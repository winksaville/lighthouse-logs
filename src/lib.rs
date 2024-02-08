use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

// The nominal mininal line length is 1 because it ends with a LF (0x0A)
// returns 0 if EOF.
pub fn process_line(line_number: usize, line: &str) -> usize {
    let len = line.len();
    if len != 0 {
        println!("{line_number} {len}: {line}");
    }

    len
}

pub fn process(reader: &mut BufReader<File>, fname: &str) -> std::io::Result<()> {
    let mut line = String::new();

    let mut line_number = 0;
    loop {
        // See: https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line
        // where it says:
        // "This function is blocking and should be used carefully: it is possible
        // for an attacker to continuously send bytes without ever sending a newline
        // or EOF. You can use take to limit the maximum number of bytes read."
        match reader.read_line(&mut line) {
            Ok(_) => {
                line_number = line_number + 1;
                let len = process_line(line_number, &line);
                if len == 0 {
                    return Ok(());
                }
                line.clear();
            }
            Err(e) => {
                println!("Error reading \"{fname}\": {e}");
                return Err(e);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_process() {
        let fname = "data/log.txt";
        let f = match File::open(fname) {
            Ok(fr) => fr,
            Err(e) => {
                println!("Could not open \"{fname}\": {e}");
                return;
            }
        };
        let mut reader = BufReader::new(f);
        process(&mut reader, fname).unwrap();
    }

    #[test]
    fn test_len_0() {
        assert_eq!(process_line(1, ""), 0);
    }
}
