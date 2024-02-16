use std::fs::File;
use std::process::ExitCode;

use custom_logger::env_logger_init;
use lighthouse_logs_lib::ReadTruncatedLines;
use log::{debug, error};

fn main() -> ExitCode {
    env_logger_init("error");
    debug!("main:+");

    // Open file
    let fname = "data/log.txt";
    let f = match File::open(fname) {
        Ok(fr) => fr,
        Err(e) => {
            error!("Could not open \"{fname}\": {e}");
            return 1.into();
        }
    };

    // Proccess the file
    let reader = std::io::BufReader::new(f);
    let mut rtl = ReadTruncatedLines::new(reader, fname, 1024);

    while let Some(line) = rtl.read_truncated_line() {
        println!("{}", line);
    }

    debug!("main:-");

    0.into()
}
