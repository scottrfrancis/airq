use chrono::Local;
// use itertools::join;
use std::env;
use std::fs::File;
use std::io;
use std::io::Read;
use std::{thread, time};



use crate::payload::Payload;

mod config;
mod grove_rgb_lcd;
use grove_rgb_lcd::GroveRgbLcd;
mod payload;


fn write_to_display(disp: &mut GroveRgbLcd, data: &str) -> ()
{
    let date = Local::now();
    let t = format!("{}\n  {}", data, date.format("%d %b %H:%M"));
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
fn set_display_color_for_aqi(disp: &mut GroveRgbLcd, aqi_level: u32) -> ()
{
    let (r, g, b) = match aqi_level
    {
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
fn main() -> io::Result<()> {
    let mut aqi_avg: u32 = 0;

    let mut display: GroveRgbLcd = grove_rgb_lcd::connect()?;
    display.set_rgb((0x10, 0x10, 0x40))?;
    
    let args: Vec<String> = env::args().collect();

    let mut f = File::open(config::parse_config(&args))?;
    let mut d = [0; 2*CHUNK_SIZE];

    let mut total_read = 0;
    loop {
        total_read += f.read(&mut d[total_read..])?;

        if total_read < CHUNK_SIZE {
            continue;
        }

        let p;
        let found;
        (_, p, found) = match payload::parse_stream_to_payload(&d) {
            Ok((i, p)) => (i, p, true),
            Err(_e) => (&d[..], Payload::default(), false),
        };

        let mut data_str = String::from("");
        if found {
            aqi_avg = aqi(p.data[1] as f64, p.data[2] as f64) as u32;
            data_str = format!("AQI {} ({},{},{})", 
               aqi_avg, p.data[0], p.data[1], p.data[2]);
        }

        println!("sending {}", data_str);
        write_to_display(&mut display, &data_str.as_str());
        set_display_color_for_aqi(&mut display, aqi_avg);

        thread::sleep(time::Duration::from_millis(3000));
    }

    // Ok(())
}
