use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::process::ExitCode;

use lighthouse_logs_lib::process;

fn main() -> ExitCode {
    // Open file
    let fname = "data/log.txt";
    let f = match File::open(fname) {
        Ok(fr) => fr,
        Err(e) => {
            println!("Could not open \"{fname}\": {e}");
            return 1.into();
        }
    };

    // Create a buffer reader
    let mut reader = BufReader::new(f);

    // Be sure the reader is at the beginning
    if let Err(e) = reader.seek(std::io::SeekFrom::Start(0)) {
        println!("Could seek to beginning of \"{fname}\": {e}");
        return 2.into();
    }

    // Proccess the stream and create an ExitCode
    match process(&mut reader, fname) {
        Ok(_) => 0.into(),
        Err(_) => 2.into(),
    }
}
