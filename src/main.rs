use std::env;
// use std::io;
// use std::fs;

mod config;
mod payload;


fn main() {
    let args: Vec<String> = env::args().collect();

    let filename= config::parse_config(&args);
    println!("reading file: {}", filename);
    // let contents = fs::read_to_string(filename)
    //     .expect("error reading file");
    // println!("\n{}\n", contents);

    // let data = fs::read(filename)
    //     .expect("bad file");
    
    // let mut start = 0;
    // loop {
    //     let x = data.get(start);
    //     if x == Some(0x42 as u8) {
    //         println!("start");
    //         break;
    //     }
        
    //     start += 1;
    // }

}
