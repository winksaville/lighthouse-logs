use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;

pub fn process(reader: &mut BufReader<File>, fname: &str) -> std::io::Result<()> {
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

