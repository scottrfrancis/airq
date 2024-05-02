use chrono::Local;
use gpio_am2302_rs::{ 
    try_read,
};
use std::{
    collections::HashMap,
    convert::TryInto,
    env,
    fs::{ 
        // read,
        File
    },
    future,
    io::Read,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};
// use futures::executor::block_on;


use crate::payload::Payload;

mod config;
mod grove_rgb_lcd;
use grove_rgb_lcd::GroveRgbLcd;
mod payload;



use tokio::net::TcpListener;

use tokio_modbus::{
    prelude::*,
    server::tcp::{accept_tcp_connection, Server},
};

struct ModbusService {
    input_registers: Arc<Mutex<HashMap<u16, u16>>>,
    holding_registers: Arc<Mutex<HashMap<u16, u16>>>,
}

impl tokio_modbus::server::Service for ModbusService {
    type Request = Request<'static>;
    type Future = future::Ready<Result<Response, Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        match req {
            Request::ReadInputRegisters(addr, cnt) => {
                 future::ready(
                    register_read(&self.input_registers.lock().unwrap(), addr, cnt)
                        .map(Response::ReadInputRegisters),
                )
            },
            Request::ReadHoldingRegisters(addr, cnt) => future::ready(
                register_read(&self.holding_registers.lock().unwrap(), addr, cnt)
                    .map(Response::ReadHoldingRegisters),
            ),
            Request::WriteMultipleRegisters(addr, values) => future::ready(
                register_write(&mut self.holding_registers.lock().unwrap(), addr, &values)
                    .map(|_| Response::WriteMultipleRegisters(addr, values.len() as u16)),
            ),
            Request::WriteSingleRegister(addr, value) => future::ready(
                register_write(
                    &mut self.holding_registers.lock().unwrap(),
                    addr,
                    std::slice::from_ref(&value),
                )
                .map(|_| Response::WriteSingleRegister(addr, value)),
            ),
            _ => {
                println!("SERVER: Exception::IllegalFunction - Unimplemented function code in request: {req:?}");
                future::ready(Err(Exception::IllegalFunction))
            }
        }
    }
}

impl ModbusService {
    fn new(readings: Arc<Mutex<HashMap<u16, u16>>>) -> Self {
        let mut holding_registers = HashMap::new();
        holding_registers.insert(0, 0);
        holding_registers.insert(1, 0);
        holding_registers.insert(2, 0);
        holding_registers.insert(3, 0);

        Self {
            input_registers: readings,
            holding_registers: Arc::new(Mutex::new(holding_registers)),
        }
    }
}

/// Helper function implementing reading registers from a HashMap.
fn register_read(
    registers: &HashMap<u16, u16>,
    addr: u16,
    cnt: u16,
) -> Result<Vec<u16>, Exception> {
    let mut response_values = vec![0; cnt.into()];
    for i in 0..cnt {
        let reg_addr = addr + i;
        if let Some(r) = registers.get(&reg_addr) {
            response_values[i as usize] = *r;
        } else {
            println!("SERVER: Exception::IllegalDataAddress");
            return Err(Exception::IllegalDataAddress);
        }
    }

    Ok(response_values)
}

/// Write a holding register. Used by both the write single register
/// and write multiple registers requests.
fn register_write(
    registers: &mut HashMap<u16, u16>,
    addr: u16,
    values: &[u16],
) -> Result<(), Exception> {
    for (i, value) in values.iter().enumerate() {
        let reg_addr = addr + i as u16;
        if let Some(r) = registers.get_mut(&reg_addr) {
            *r = *value;
        } else {
            println!("SERVER: Exception::IllegalDataAddress");
            return Err(Exception::IllegalDataAddress);
        }
    }

    Ok(())
}

fn write_to_display(disp: &mut GroveRgbLcd, data: &str) -> ()
{
    let date = Local::now();
    let t = format!("{}\n    {}", data, date.format("%d %b %H:%M"));
    match disp.set_text(&t)
    {
        Err(err) => { println!("error: {:?}", err);},
        Ok(_) => {},
    }

    ()
}

// CF: https://forum.airnowtech.org/t/the-aqi-equation/169
// computes AQI for PM 2.5 concentrations and PM 10, then 
// averages them for an aggregate number
fn aqi(conc_2_5: f64, conc_10: f64) -> f64
{
    // First use PM 2.5 in ug/m^3
    let (conc_lo, conc_hi, aqi_lo, aqi_hi) = match conc_2_5
    {
        x if x <  12.0 => (  0.0,  12.0,   0.0,  50.0),
        x if x <  35.4 => ( 12.1,  35.4,  51.0, 100.0),
        x if x <  55.4 => ( 35.5,  55.4, 101.0, 150.0),
        x if x < 150.4 => ( 55.5, 150.4, 151.0, 200.0),
        x if x < 250.4 => (150.5, 250.4, 201.0, 300.0),
        x if x < 500.4 => (250.5, 500.4, 301.0, 500.0),
        
        _ => (500.5, 1000.0, 501.0, 1000.0),
    };
    let aqi_2_5 = ((aqi_hi - aqi_lo)/(conc_hi - conc_lo))*
                    (conc_2_5 - conc_lo) +
                    aqi_lo;

    // Now use PM 10 in ug/m^3
    let (conc_lo, conc_hi, aqi_lo, aqi_hi) = match conc_10
    {
        x if x <  54.0 => (  0.0,  54.0,   0.0,  50.0),
        x if x < 154.0 => ( 55.0, 154.0,  51.0, 100.0),
        x if x < 254.0 => (155.0, 254.0, 101.0, 150.0),
        x if x < 354.0 => (255.0, 354.0, 151.0, 200.0),
        x if x < 424.0 => (355.0, 424.0, 201.0, 300.0),
        x if x < 604.0 => (425.0, 604.0, 301.0, 500.0),
        
        _ => (605.0, 1000.0, 501.0, 1000.0),
    };
    let aqi_10 = ((aqi_hi - aqi_lo)/(conc_hi - conc_lo))*
                    (conc_10 - conc_lo) +
                    aqi_lo;

    (aqi_2_5 + aqi_10)/2.0
}

// CF - https://www.epa.gov/sites/default/files/2014-05/documents/zell-aqi.pdf
fn set_display_color_for_aqi(disp: &mut GroveRgbLcd, aqi_level: u16) -> ()
{
    let (r, g, b) = match aqi_level
    {
        x if x <=  15 => (0x00, 0x10, 0x40),    // light blue
        x if x <=  50 => (0x00, 0x80, 0x00),    // green
        x if x <= 100 => (0x80, 0x80, 0x00),    // yellow
        x if x <= 150 => (0xF0, 0x40, 0x00),    // orange
        x if x <= 200 => (0xF0, 0x00, 0x00),    // red
        x if x <= 300 => (0xA0, 0x00, 0x40),    // purple
        
        _ => (0xFF, 0x00, 0xFF)     // maroon
    };
    match disp.set_rgb((r,g,b))
    {
        _ => {},
    }

    ()
}

const CHUNK_SIZE: usize = 64;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>  {
    let socket_addr: SocketAddr = "0.0.0.0:5502".parse().unwrap();

    // use readings to hold the last 3 readings in a register format for the modbus server
    let mut readings: HashMap<u16, u16> = HashMap::new();
    readings.insert(0, 0);      // PM 1.0
    readings.insert(1, 0);    // PM 2.5
    readings.insert(2, 0);  // PM 10
    readings.insert(3, 0);  // temp deg K*100
    readings.insert(4, 0);  // humidity percent * 100
    readings.insert(5, 0);  // AQI
    readings.insert(6, 0);  // AQI update tick - LW
    readings.insert(7, 0);
    readings.insert(8, 0);  
    readings.insert(9, 0);  // AQI update tick - HW
    readings.insert(10, 0);  // temp/hum update tick - HW
    readings.insert(11, 0);  
    readings.insert(12, 0);  
    readings.insert(13, 0);  // temp/hum update tick - LW

    let readings = Arc::new(Mutex::new(readings));

    let r1 = readings.clone();
    thread::spawn(move || {
        sampling_context(r1);
    });

    let r2 = readings.clone();
    thread::spawn(move || {
        temp_humidity_sampling(r2);
    });

    tokio::select! {
        _ = server_context(socket_addr, readings.clone()) => unreachable!(),
    }

    // Ok(())
}

// temp and humidity sampling
fn temp_humidity_sampling(readings: Arc<Mutex<HashMap<u16, u16>>>) {
    let gpio_number = 4;
    loop {
        match try_read(gpio_number) {
            Ok(reading) => {
                println!("{:.1}°C,{:.1}%", 
                    reading.temperature, reading.humidity);

                let mut registers = readings.lock().unwrap();
                if 14 <= registers.len() {
                    let x = registers.get_mut(&3).unwrap();
                    *x = (reading.temperature*100.0 + 27315.0) as u16;
                    let x = registers.get_mut(&4).unwrap();
                    *x = (reading.humidity*100.0) as u16;

                    let ticks: u64 = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let x = registers.get_mut(&10).unwrap();
                    *x = (ticks >> 48) as u16;
                    let x = registers.get_mut(&11).unwrap();
                    *x = ((ticks >> 32) & 0xFFFF) as u16;
                    let x = registers.get_mut(&12).unwrap();
                    *x = ((ticks >> 16) & 0xFFFF) as u16;
                    let x = registers.get_mut(&13).unwrap();
                    *x = (ticks & 0xFFFF) as u16;
                }
            },
            _ => { },
        }
        thread::sleep(Duration::from_secs(10));
    }
}

// async 
fn sampling_context(readings: Arc<Mutex<HashMap<u16, u16>>>) {
    let mut display = grove_rgb_lcd::connect().unwrap();
    let _ = display.set_rgb((0x10, 0x10, 0x40));
    
    let args: Vec<String> = env::args().collect();

    let mut f = File::open(config::parse_config(&args)).unwrap();
    let mut d = [0; 2*CHUNK_SIZE];

    let mut total_read = 0;

    loop {
        let now = SystemTime::now();
            // .duration_since(UNIX_EPOCH)
            // .as_millis();

        total_read += f.read(&mut d[total_read..]).unwrap_or(0);

        if total_read < CHUNK_SIZE {
            continue;
        }

        let p;
        let found;
        (_, p, found) = match payload::parse_stream_to_payload(&d) {
            Ok((i, p)) => (i, p, true),
            Err(_e) => (&d[..], Payload::default(), false),
        };

        if found {
            let aqi_avg = aqi(p.data[1] as f64, p.data[2] as f64) as u16;
            let data_str = format!("AQI {} ({},{},{})", 
               aqi_avg, p.data[0], p.data[1], p.data[2]);

            println!("{},{},{}", p.data[0], p.data[1], p.data[2]);
            // update the readings register
            let mut readings = readings.lock().unwrap();
            for i in 0..3 {
                let addr = i as u16;
                if i < readings.len() {
                    let x = readings.get_mut(&addr).unwrap();
                    *x = p.data[i];
                }
                else {
                    readings.insert(addr, p.data[i]);
                }
            }
            let x = readings.get_mut(&5).unwrap();
            *x = aqi_avg;

            let ticks: u64 = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let x = readings.get_mut(&6).unwrap();
            *x = (ticks >> 48) as u16;
            let x = readings.get_mut(&7).unwrap();
            *x = ((ticks >> 32) & 0xFFFF) as u16;
            let x = readings.get_mut(&8).unwrap();
            *x = ((ticks >> 16) & 0xFFFF) as u16;
            let x = readings.get_mut(&9).unwrap();
            *x = (ticks & 0xFFFF) as u16;

            write_to_display(&mut display, &data_str.as_str());
            set_display_color_for_aqi(&mut display, aqi_avg);

            total_read = 0;
            d = [0; 2*CHUNK_SIZE];
        }

        // sensor updates every 1000 mS, so this will grab doubles,
        // but will ensure ALL updates are captured 
        // AND the clock will run smoothly...
        let elapsed_millis: u64 = now.elapsed()
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap();

        thread::sleep(Duration::from_millis(1*1000 - elapsed_millis - 10));
    }
}

async fn server_context(socket_addr: SocketAddr, readings: Arc<Mutex<HashMap<u16, u16>>>) -> anyhow::Result<()> {
    // loop {
    println!("Starting up Modbus server on {socket_addr}");
    let listener = TcpListener::bind(socket_addr).await?;
    // let bind_result = block_on(TcpListener::bind(socket_addr));
    // let listener = bind_result.unwrap();

    let server = Server::new(listener);
    let new_service = |_socket_addr| Ok(Some(ModbusService::new(readings.clone())));
    let on_connected = |stream, socket_addr| async move {
        accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    println!("ready to serve");
    server.serve(&on_connected, on_process_error).await?;
    // let _ = block_on(server.serve(&on_connected, on_process_error));
    println!("Server done");
    // }
    Ok(())
}
