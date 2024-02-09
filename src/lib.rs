use std::io::{prelude::*, BufReader, Read, Seek, Error, ErrorKind};
use log::debug;

pub trait MyTrait: std::io::Read + Seek {}

// The nominal mininal line length is 1 because it ends with a LF (0x0A)
// returns 0 if EOF.
pub fn process_line(line_number: usize, line: &str) -> usize {
    let len = line.len();
    debug!("process_line {line_number}:- len={len} line=\"{line}\"");
    len
}

pub fn process<R: Read>(
    rdr: R,
    _fname: &str,
    max_line_length: u64,
) -> std::io::Result<(usize, usize)> {
    // line_count and max_processed_line_len
    let mut line = String::with_capacity(max_line_length as usize);
    let mut beginning_of_line = String::with_capacity(max_line_length as usize);
    let mut full_line_len = 0usize;
    let mut line_number = 0usize;
    let mut max_processed_line_len = 0usize;

    // See: https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line
    // where it says:
    //   "This function is blocking and should be used carefully: it is possible
    //    for an attacker to continuously send bytes without ever sending a newline
    //    or EOF. You can use take to limit the maximum number of bytes read."
    //
    // At first I couldn't get using `take()` to compile it was complaining that
    // the reader was moving. Here[1] was the key, I needed to use `by_ref().take(max)`
    // to be able to use in the loop and not an error. This then allows me to take the
    // max_line_length bytes from each line and ignore the rest.
    //
    // [1]:https://users.rust-lang.org/t/idiomatic-way-of-reading-lines-in-a-safe-manner/62942
    //  
    let mut reader = BufReader::with_capacity(max_line_length as usize, rdr);

    #[derive(Debug, PartialEq)]
    enum State {
        BeginningOfLine,
        DiscardUntilLF,
        ProcessBeginningOfLine,
    }
    let mut state: State = State::BeginningOfLine;

    while let Ok(orig_len) = reader.by_ref().take(max_line_length).read_line(&mut line) {

        debug!("TOL: state={state:?} line_number={line_number} orig_len={orig_len} line=\"{line}\"");

        match state {
            State::BeginningOfLine => {
                if line.ends_with("\n") {
                    debug!("BeginningOfLine {line_number}: state = ProcessBeginningOfLine");

                    // We have a complete line, remove the LF
                    line.pop();
                    beginning_of_line = line.clone();
                    state = State::ProcessBeginningOfLine;
                } else {
                    // It might be too long or it's the last line and there is no-lf.
                    // Either way it will be handled properly in DiscardUntilLF.
                    beginning_of_line = line.clone();
                    debug!("BeginningOfLine {line_number}: state = DiscardUntilLF");
                    state = State::DiscardUntilLF;
                }
                full_line_len += beginning_of_line.len();
            }
            State::DiscardUntilLF => {
                if line.ends_with("\n") {
                    full_line_len += orig_len - 1;
                    state = State::ProcessBeginningOfLine;
                    debug!("DiscardUntilLF {line_number}: LF, done discarding");
                } else if orig_len == 0 {
                    state = State::ProcessBeginningOfLine;
                    debug!("DiscardUntilLF {line_number}: EOF, done");
                } else {
                    full_line_len += orig_len;
                    debug!("DiscardUntilLF {line_number}: no LF or EOF, continue to discard");
                }
            }
            State::ProcessBeginningOfLine => {
                return Err(Error::new(ErrorKind::Other, "State::ProcessBeginningOfLine should not occur"));
            }
        }

        if state == State::ProcessBeginningOfLine {
            // Arrive here if we've got a complete line and we need
            // to ProcessBeginningOfLine or it's EOF.
            line_number += 1;
            let line_len = beginning_of_line.len();
            debug!("ProcessBeginningOfLine {line_number}: line_len={line_len}");
            assert!(line_len == process_line(line_number, &beginning_of_line)); //.trim_end_matches('\n')));

            // Remember the longest line we've processed
            if full_line_len > max_processed_line_len {
                max_processed_line_len = full_line_len;
            }

            full_line_len = 0;
            state = State::BeginningOfLine;
        }

        if orig_len == 0 {
            // If we read 0 bytes, we are at EOF
            debug!("EOF: {line_number} lines, max line length={max_processed_line_len}");
            break;
        }

        line.clear();
    }

    Ok((line_number, max_processed_line_len))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::io::Cursor;
    use test_log::test;
    use log::error;

    #[test]
    #[should_panic] // Comment out and you will see the error message
    fn test_error_and_panic() {
        error!("Error prior to panic");
        panic!();
    }

    #[test]
    fn test_process() {
        let fname = "data/log.txt";
        let f = match File::open(fname) {
            Ok(fr) => fr,
            Err(e) => {
                error!("Could not open \"{fname}\": {e}");
                panic!();
            }
        };
        let mut reader = BufReader::new(f);
        let (lines, max_line_len) = process(&mut reader, fname, 1024).unwrap();
        assert_eq!(lines, 4);
        assert_eq!(max_line_len, 11);
    }

    #[test]
    fn test_len_0() {
        let data = b"".to_vec();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 0);
        assert_eq!(max_line_len, 0);
    }

    #[test]
    fn test_len_1_no_lf() {
        let data = b"1".to_vec();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 1);
        assert_eq!(max_line_len, 1);
    }

    #[test]
    fn test_len_equals_max_line_length_1() {
        let data = b"1".to_vec();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 1).expect("work");
        assert_eq!(lines, 1);
        assert_eq!(max_line_len, 1);
    }

    #[test]
    fn test_len_equals_max_line_length_1_with_lf() {
        let data = b"1\n".to_vec();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 1).expect("work");
        assert_eq!(lines, 1);
        assert_eq!(max_line_len, 1);
    }

    #[test]
    fn test_len_equals_max_line_length_10() {
        let data = b"0123456789".to_vec();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 1);
        assert_eq!(max_line_len, 10);
    }

    #[test]
    fn test_len_equals_max_line_length_10_with_lf() {
        let data = b"0123456789\n".to_vec();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 1);
        assert_eq!(max_line_len, 10);
    }

    #[test]
    fn test_len_1_just_lf() {
        let data = b"\n".to_vec();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 1);
        assert_eq!(max_line_len, 0);
    }

    #[test]
    fn test_one_long_line() {
        let data = b"This is a line that is too long to fit in the buffer".to_vec();
        let expected_max_line_len = data.len();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 1);
        assert_eq!(expected_max_line_len, max_line_len);
    }

    #[test]
    fn test_two_long_lines() {
        let line1 = b"First line that is too long to fit in the buffer\n".to_vec();
        let line2 = b"Second line that shorter to still too long".to_vec();
        let expected_max_line_len = line1.len() - 1;
        let data = [line1, line2].concat();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 2);
        assert_eq!(expected_max_line_len, max_line_len);
    }

    #[test]
    fn test_three_long_lines() {
        let line1 = b"First line that is too long to fit in the buffer\n".to_vec();
        let line2 = b"Second line that shorter to still too long\n".to_vec();
        let line3 = b"Thrid line that shorter to still too long\n".to_vec();
        let expected_max_line_len = line1.len() - 1;
        let data = [line1, line2, line3].concat();
        let mut reader = Cursor::new(data);
        let (lines, max_line_len) = process(&mut reader, "test", 10).expect("work");
        assert_eq!(lines, 3);
        assert_eq!(expected_max_line_len, max_line_len);
    }
}
