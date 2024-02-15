use std::io::{prelude::*, BufReader, Read, Seek};
use log::debug;

pub trait MyTrait: std::io::Read + Seek {}

pub struct ReadTruncatedLines<R: Read> {
    reader: BufReader<R>,
    reader_id: String,
    capacity: usize,
    max_line_length: u64,
    full_line_len: usize,
    line: String,
    discard_line: String,
    line_number: usize,
    max_processed_full_line_len: usize,
}

impl<R: Read> ReadTruncatedLines<R> {
    pub fn new(
        reader: R,
        reader_id_like_file_name: &str,
        capacity: usize,
        max_line_length: u64,
    ) -> Self {
        ReadTruncatedLines {
            reader: BufReader::with_capacity(capacity, reader),
            reader_id: reader_id_like_file_name.to_string(),
            capacity,
            max_line_length,
            full_line_len: 0,
            line: String::with_capacity(max_line_length as usize),
            discard_line: String::with_capacity(max_line_length as usize),
            line_number: 0,
            max_processed_full_line_len: 0,
        }
    }

    pub fn read_truncated_line(&mut self) -> Option<&str> {
        //<ReadTruncatedLines<R> as IntoIterator>::Item> {
        self.line.clear();
        match self
            .reader
            .by_ref()
            .take(self.max_line_length)
            .read_line(&mut self.line)
        {
            Ok(orig_len) => {
                let mut too_long = false;
                if orig_len == 0 {
                    // If we read 0 bytes, we are at EOF
                    debug!("{}: EOF", self.line_number);
                    return None;
                } else if self.line.ends_with("\n") {
                    // We have a complete line, remove the LF
                    self.line.pop();
                } else {
                    // It might be too long or it's the last line and there is no-lf.
                    // Either way it will be handled properly in too_long loop.
                    debug!("Line {} is too long", self.line_number);
                    too_long = true;
                }

                self.line_number += 1;
                self.full_line_len = self.line.len();
                debug!("{}, line_len={}", self.line_number, self.line.len());

                if too_long {
                    // Loop until we find the end of the line, ignoring the rest
                    let mut ignore_loops = 0;
                    loop {
                        self.discard_line.clear();

                        match self
                            .reader
                            .by_ref()
                            .take(self.max_line_length)
                            .read_line(&mut self.discard_line)
                        {
                            Ok(len) => {
                                // Update current line length and max_processed_line_len

                                if len == 0 {
                                    debug!("ignore_loop: {ignore_loops}: EOF ignoring, full_line_len={}", self.full_line_len);
                                    break;
                                } else if self.line.ends_with("\n") {
                                    debug!("ignore_loop: {ignore_loops}: LF end of line ignoring, full_line_len={}", self.full_line_len);
                                    self.line.pop();
                                    self.full_line_len += self.line.len();
                                    break;
                                } else {
                                    self.full_line_len += len;
                                    debug!("ignore_loop: {ignore_loops}: line_number={}, ignoring len={} full_line_len={}", self.line_number, len, self.full_line_len);
                                }
                            }
                            Err(e) => {
                                debug!(
                                    "ignore_loop: {ignore_loops}: error reading \"{}\": {e}",
                                    self.reader_id
                                );

                                // TODO, we should probably return the line count and max line length
                                return None;
                            }
                        }
                        ignore_loops += 1;
                    }
                }

                debug!(
                    "{}, line_len={} full_line_len={}",
                    self.line_number,
                    self.line.len(),
                    self.full_line_len
                );

                // Remember the longest line we've processed
                if self.full_line_len > self.max_processed_full_line_len {
                    self.max_processed_full_line_len = self.full_line_len;
                }

                Some(&self.line)
            }
            Err(e) => {
                debug!("Error reading \"{}\": {e}", self.reader_id);
                None
            }
        }
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }

    pub fn max_processed_full_line_len(&self) -> usize {
        self.max_processed_full_line_len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn line(&self) -> &str {
        &self.line
    }

    pub fn line_len(&self) -> usize {
        self.line.len()
    }

    pub fn full_line_len(&self) -> usize {
        self.full_line_len
    }
}

// The nominal mininal line length is 1 because it ends with a LF (0x0A)
// returns 0 if EOF.
pub fn process_line(line_number: usize, line: &str) -> usize {
    let len = line.len();
    println!("process_line {line_number}:- len={len} line=\"{line}\"");
    len
}

pub fn process<R: Read>(
    rdr: R,
    fname: &str,
    max_line_length: u64,
) -> std::io::Result<(usize, usize)> {
    // line_count and max_processed_line_len
    let mut line = String::with_capacity(max_line_length as usize);

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

    let mut line_number = 0usize;
    let mut max_processed_line_len = 0usize;
    loop {
        match reader.by_ref().take(max_line_length).read_line(&mut line) {
            Ok(orig_len) => {
                line_number += 1;

                let mut too_long = false;
                if orig_len == 0 {
                    // If we read 0 bytes, we are at EOF
                    debug!("{line_number}: EOF");
                } else if line.ends_with("\n") {
                    // We have a complete line, remove the LF
                    line.pop();
                } else {
                    // It might be too long or it's the last line and there is no-lf.
                    // Either way it will be handled properly in too_long loop.
                    debug!("Line {line_number} is too long");
                    too_long = true;
                }

                let mut line_len = line.len();
                debug!("{line_number}, line_len={line_len}");

                // Process the line
                assert!(line_len == process_line(line_number, &line));

                if too_long {
                    // Loop until we find the end of the line, ignoring the rest
                    let mut ignore_loops = 0;
                    loop {
                        line.clear();

                        match reader.by_ref().take(max_line_length).read_line(&mut line) {
                            Ok(len) => {
                                // Update current line length and max_processed_line_len

                                if len == 0 {
                                    debug!("ignore_loop: {ignore_loops}: EOF ignoring, line_len={line_len}");
                                    break;
                                } else if line.ends_with("\n") {
                                    debug!("ignore_loop: {ignore_loops}: LF end of line ignoring, line_len={line_len}");
                                    line.pop();
                                    line_len += line.len();
                                    break;
                                } else {
                                    line_len += len;
                                    debug!("ignore_loop: {ignore_loops}: line_number={line_number}, ignoring len={len} line_len={line_len}");
                                }
                            }
                            Err(e) => {
                                debug!("ignore_loop: {ignore_loops}: error reading \"{fname}\": {e}");

                                // TODO, we should probably return the line count and max line length
                                return Err(e);
                            }
                        }
                        ignore_loops += 1;
                    }
                }

                debug!("{line_number}, line_len={line_len}");

                // Remember the longest line we've processed
                if line_len > max_processed_line_len {
                    max_processed_line_len = line_len;
                }

                if orig_len == 0 {
                    line_number -= 1;
                    // If we read 0 bytes, we are at EOF
                    debug!("EOF: {line_number} lines, max line length={max_processed_line_len}");
                    return Ok((line_number, max_processed_line_len));
                }

                // Clear the line for the next read
                line.clear();
            }
            Err(e) => {
                debug!("Error reading \"{fname}\": {e}");
                return Err(e);
            }
        }
    }
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
    fn test_read_truncated_lines() {
        let expected_lines = vec!["line 1", "line 2", "", "last line 4"];
        let fname = "data/log.txt";
        let f = match File::open(fname) {
            Ok(fr) => fr,
            Err(e) => {
                error!("Could not open \"{fname}\": {e}");
                panic!();
            }
        };
        let mut reader = BufReader::new(f);
        let mut rtl = ReadTruncatedLines::new(&mut reader, fname, 1024, 1024);

        for expected_line in expected_lines.into_iter() {
            let line = rtl.read_truncated_line().unwrap();
            println!("{line}");
            assert_eq!(line, expected_line);
        }
        assert_eq!(rtl.line_number(), 4);
        assert_eq!(rtl.max_processed_full_line_len(), 11);
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
