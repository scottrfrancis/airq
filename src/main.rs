use chrono::Local;
use gpio_am2302_rs::{ try_read };
use std::{
    collections::HashMap,
    env,
    fs::{ File },
    future,
    io::Read,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

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
        let mut holding_registers = HashMap::with_capacity(16);
        for k in 0..16 {
            holding_registers.insert(k, 0);
        }

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

fn read_float_register(registers: &HashMap<u16, u16>, addr: u16) -> f32 {
    let u = (read_register(registers, addr) as u32) << 16 | 
                    (read_register(registers, addr + 1) as u32);
    let f = f32::from_bits(u);
    f
}
fn write_float_register(registers: &mut HashMap<u16, u16>, addr: u16, f: f32) {
    let u = f.to_bits() as u32;
    write_register(registers, addr, (u >> 16) as u16);
    write_register(registers, addr + 1, (u & 0xFFFF) as u16);
} 

#[allow(dead_code)]
fn read_long_register(registers: &HashMap<u16, u16>, addr: u16) -> u32 {
    (read_register(registers, addr) as u32) << 16 | 
        (read_register(registers, addr + 1) as u32)
}
fn write_long_register(registers: &mut HashMap<u16, u16>, addr: u16, u: u32) {
    let hw = (u >> 16) as u16;
    let lw = (u & 0xFFFF) as u16;
    write_register(registers, addr, hw);
    write_register(registers, addr + 1, lw);
}

fn read_register(registers: &HashMap<u16, u16>, addr: u16) -> u16 {
    match registers.get(&addr) {
        Some(x) => *x,
        None => 0,
    }
}
fn write_register(registers: &mut HashMap<u16, u16>, addr: u16, value: u16) {
    if !registers.contains_key(&addr) {
        registers.insert(addr, value);
    } else {
        let x = registers.get_mut(&addr).unwrap();
        *x = value;
    }
}

// indexes into the readings register
const AQI: u16 = 0;
const PM_1_0: u16 = 1;
const PM_2_5: u16 = 2;
const PM_10: u16 = 3;
const AQI_TICK_HW: u16 = 4;
const TEMP_HW: u16 = 6;
const HUM_HW: u16 = 8;
const TEMP_HUM_TICK_HW: u16 = 10;

const CHUNK_SIZE: usize = 64;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>  {
    let socket_addr: SocketAddr = "0.0.0.0:5502".parse().unwrap();

    // use readings to hold the last 3 readings in a register format for the modbus server
    let mut registers: HashMap<u16, u16> = HashMap::with_capacity(16);
    // default them to 0
    write_register(&mut registers, AQI, 0);         // 1 * 16-bit
    write_register(&mut registers, PM_1_0, 0);      // 1
    write_register(&mut registers, PM_2_5, 0);      // 1
    write_register(&mut registers, PM_10, 0);    // 1
    write_long_register(&mut registers, AQI_TICK_HW, 0); // 2 * 16-bit

    write_float_register(&mut registers, TEMP_HW, -40.0); // 2 
    write_float_register(&mut registers, HUM_HW, 0.0);  // 2
    write_long_register(&mut registers, TEMP_HUM_TICK_HW, 0); // 2 * 16-bit
    // 12 registers * 16-bit = 24 bytes
    
    let readings = Arc::new(Mutex::new(registers));

    let r1 = readings.clone();
    thread::spawn(move || {
        sampling_context(r1);
    });

    let r2 = readings.clone();
    thread::spawn(move || {
        temp_humidity_sampling(r2);
    });

    // add a display output thread
    let r3 = readings.clone();
    thread::spawn(move || {
        display_registers(r3);
});

    tokio::select! {
        _ = server_context(socket_addr, readings.clone()) => unreachable!(),
    }
}

fn display_registers(readings: Arc<Mutex<HashMap<u16, u16>>>) {
    let mut display = grove_rgb_lcd::connect().unwrap();
    let _ = display.set_rgb((0x10, 0x10, 0x40));

    write_to_display(&mut display, &"");
    
    loop {
        thread::sleep(Duration::from_secs(30));     // wait for the first reading to come in

        // lines are 16 chars long
        // "AQI xx xx.x° xx%"
        let registers = readings.lock().unwrap();
        let aqi = read_register(&*registers, AQI);
        // let deg = 0xDF as char;
        let deg = 'F';  // for now just use F -- the char isn't showing up as per datasheet
        let t = read_float_register(&*registers, TEMP_HW) * 9.0/5.0 + 32.0;
        let h = read_float_register(&*registers, HUM_HW) as u16;
        drop(registers);

        let line1 = format!("AQI {} {:.1}{} {}%", aqi, t, deg, h);

        write_to_display(&mut display, &line1);
        set_display_color_for_aqi(&mut display, aqi);
    }
}

// temp and humidity sampling
const GPIO_NUMBER: u32 = 4;
fn temp_humidity_sampling(readings: Arc<Mutex<HashMap<u16, u16>>>) {
    loop {
        match try_read(GPIO_NUMBER) {
            Ok(reading) => {
                println!("{:.1}°C,{:.1}%", 
                    reading.temperature, reading.humidity);

                let mut registers = readings.lock().unwrap();
                write_float_register(&mut *registers, TEMP_HW, reading.temperature);
                write_float_register(&mut *registers, HUM_HW, reading.humidity);

                let ticks: u64 = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                write_long_register(&mut *registers, TEMP_HUM_TICK_HW, (ticks & 0xFFFFffff) as u32);
                drop(registers);
            },
            _ => { },
        }
        thread::sleep(Duration::from_secs(10));
    }
}

fn sampling_context(readings: Arc<Mutex<HashMap<u16, u16>>>) {  
    let args: Vec<String> = env::args().collect();

    let mut f = File::open(config::parse_config(&args)).unwrap();
    let mut d = [0; 2*CHUNK_SIZE];

    let mut total_read = 0;

    loop {
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
            println!("{},{},{}", p.data[0], p.data[1], p.data[2]);
            // update the readings registers
            let mut registers = readings.lock().unwrap();
            write_register(&mut *registers, AQI, aqi_avg);
            write_register(&mut *registers, PM_1_0, p.data[0] as u16);
            write_register(&mut *registers, PM_2_5, p.data[1] as u16);
            write_register(&mut *registers, PM_10, p.data[2] as u16);

            let ticks: u64 = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            write_long_register(&mut *registers, AQI_TICK_HW, (ticks & 0xFFFFffff) as u32);
            drop(registers);

            total_read = 0;
            d = [0; 2*CHUNK_SIZE];
        }

        thread::sleep(Duration::from_secs(1));
    }
}

async fn server_context(socket_addr: SocketAddr, readings: Arc<Mutex<HashMap<u16, u16>>>) -> anyhow::Result<()> {
    println!("Starting up Modbus server on {socket_addr}");
    let listener = TcpListener::bind(socket_addr).await?;

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
    println!("Server done");

    Ok(())
}
