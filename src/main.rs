use std::fs::File;
use std::io::prelude::*;
use std::process::ExitCode;

use lighthouse_logs_lib::process;

fn main() -> ExitCode {
    // Open file
    let fname = "data/log.txt";
    let mut f = match File::open(fname) {
        Ok(fr) => fr,
        Err(e) => {
            println!("Could not open \"{fname}\": {e}");
            return 1.into();
        }
    };

    // Create a buffer reader
    //let mut reader = BufReader::new(f);

    // Be sure the reader is at the beginning
    if let Err(e) = f.seek(std::io::SeekFrom::Start(0)) {
        println!("Could seek to beginning of \"{fname}\": {e}");
        return 2.into();
    }

    // Proccess the stream and create an ExitCode
    match process(&f, fname, 1024) {
        Ok(_) => 0.into(),
        Err(_) => 2.into(),
    }
}
