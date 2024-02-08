use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::process::ExitCode;

fn process(reader: &mut BufReader<File>, fname: &str) -> std::io::Result<()> {
    let mut line = String::new();

    // See: https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line
    // where it says:
    // "This function is blocking and should be used carefully: it is possible
    // for an attacker to continuously send bytes without ever sending a newline
    // or EOF. You can use take to limit the maximum number of bytes read."
    let len = match reader.read_line(&mut line) {
        Ok(l) => l,
        Err(e) => {
            println!("Error reading \"{fname}\": {e}");
            return Err(e);
        }
    };
    println!("First line is {len} bytes long");

    Ok(())
}

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

