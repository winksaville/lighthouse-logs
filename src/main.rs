use std::fs::File;
use std::process::ExitCode;

use custom_logger::env_logger_init;
use log::{debug, error};

use lighthouse_logs_lib::process;

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

    // Proccess the stream and create an ExitCode
    let exit_code = match process(&f, fname, 1024) {
        Ok(_) => 0.into(),
        Err(_) => 2.into(),
    };

    debug!("main:-");

    exit_code
}
