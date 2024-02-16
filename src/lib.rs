use log::debug;
use std::io::{prelude::*, BufReader, Read, Seek};

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
    pub fn new(reader: R, reader_id_like_file_name: &str, max_line_length: u64) -> Self {
        ReadTruncatedLines {
            reader: BufReader::with_capacity(max_line_length as usize, reader),
            reader_id: reader_id_like_file_name.to_string(),
            capacity: max_line_length as usize,
            max_line_length,
            full_line_len: 0,
            line: String::with_capacity(max_line_length as usize),
            discard_line: String::with_capacity(max_line_length as usize),
            line_number: 0,
            max_processed_full_line_len: 0,
        }
    }

    pub fn read_truncated_line(&mut self) -> Option<&str> {
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
                } else if self.line.ends_with('\n') {
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
                                } else if self.discard_line.ends_with('\n') {
                                    debug!("ignore_loop: {ignore_loops}: LF end of line ignoring, full_line_len={}", self.full_line_len);
                                    self.full_line_len += self.discard_line.len() - 1;
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

#[cfg(test)]
mod test {
    use super::*;
    use log::error;
    use std::fs::File;
    use std::io::Cursor;
    use test_log::test;

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
        let mut rtl = ReadTruncatedLines::new(&mut reader, fname, 1024);

        for expected_line in expected_lines.into_iter() {
            let line = rtl.read_truncated_line().unwrap();
            println!("{line}");
            assert_eq!(line, expected_line);
        }
        assert_eq!(rtl.line_number(), 4);
        assert_eq!(rtl.max_processed_full_line_len(), 11);
    }

    #[test]
    fn test_len_0() {
        let data = b"".to_vec();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 1024);

        // On an empty file we should get None and line_number
        // and max_processed_full_line_len should not change
        assert_eq!(None, rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 0);
        assert_eq!(rtl.line_number(), 0);
        assert_eq!(rtl.max_processed_full_line_len(), 0);
        assert_eq!(None, rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 0);
        assert_eq!(rtl.line_number(), 0);
        assert_eq!(rtl.max_processed_full_line_len(), 0);
    }

    #[test]
    fn test_just_lf() {
        let data = b"\n".to_vec();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 1024);

        assert_eq!(Some(""), rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 0);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 0);

        // On an empty file we should get None and line_number
        // and max_processed_full_line_len should not change
        assert_eq!(None, rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 0);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 0);
        assert_eq!(None, rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 0);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 0);
    }

    #[test]
    fn test_len_1_no_lf() {
        let data = b"1".to_vec();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 1024);
        assert_eq!(Some("1"), rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 1);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 1);

        // Subsequent reads should return None and line_number is the number of lines total
        // and max_processed_full_line_len should not change
        assert_eq!(None, rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 0);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 1);
        assert_eq!(None, rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 0);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 1);
    }

    #[test]
    fn test_len_equals_max_line_length_1() {
        let data = b"1".to_vec();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 1);
        assert_eq!(Some("1"), rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 1);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 1);
    }

    #[test]
    fn test_len_equals_max_line_length_1_with_lf() {
        let data = b"1\n".to_vec();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 1);
        assert_eq!(Some("1"), rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 1);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 1);
    }

    #[test]
    fn test_len_equals_max_line_length_10() {
        let data = b"0123456789".to_vec();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 10);
        assert_eq!(Some("0123456789"), rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 10);
    }

    #[test]
    fn test_len_equals_max_line_length_10_with_lf() {
        let data = b"0123456789\n".to_vec();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 10);
        assert_eq!(Some("0123456789"), rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), 10);
    }

    #[test]
    fn test_one_long_line() {
        let data = b"This is a line that is too long to fit in the buffer".to_vec();
        let expected_max_line_len = data.len();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 10);
        assert_eq!(Some("This is a "), rtl.read_truncated_line());
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), expected_max_line_len);
    }

    #[test]
    fn test_two_long_lines() {
        let line1 = b"First line that is too long to fit in the buffer\n".to_vec();
        let line2 = b"Second line that shorter to still too long".to_vec();
        let expected_max_line_len = line1.len() - 1;
        let expected_line1 = std::str::from_utf8(&line1[0..10]).unwrap();
        debug!("expected_line1='{expected_line1}'");
        let expected_line2 = std::str::from_utf8(&line2[0..10]).unwrap();
        debug!("expected_line2='{expected_line2}'");

        let data = [line1.clone(), line2.clone()].concat();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 10);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line1));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), line1.len() - 1);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line2));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 2);
        assert_eq!(rtl.max_processed_full_line_len(), expected_max_line_len);
    }

    #[test]
    fn test_three_long_lines_1_is_longest() {
        let line1 = b"First line that is longest to fit in the buffer\n".to_vec();
        let line2 = b"Second line that's shorter but still too long\n".to_vec();
        let line3 = b"Thrid line that's shorter but still too long\n".to_vec();
        let expected_max_line_len = line1.len() - 1;
        let expected_line1 = std::str::from_utf8(&line1[0..10]).unwrap();
        debug!("expected_line1='{expected_line1}'");
        let expected_line2 = std::str::from_utf8(&line2[0..10]).unwrap();
        debug!("expected_line2='{expected_line2}'");
        let expected_line3 = std::str::from_utf8(&line3[0..10]).unwrap();
        debug!("expected_line3='{expected_line3}'");

        let data = [line1.clone(), line2.clone(), line3.clone()].concat();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 10);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line1));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), line1.len() - 1);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line2));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 2);
        assert_eq!(rtl.max_processed_full_line_len(), expected_max_line_len);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line3));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 3);
        assert_eq!(rtl.max_processed_full_line_len(), expected_max_line_len);
    }

    #[test]
    fn test_three_long_lines_2_is_longest() {
        let line1 = b"First line that's shorter but still too long\n".to_vec();
        let line2 = b"Second line that is longest and longer than buffer\n".to_vec();
        let line3 = b"Thrid line that's shorter but still too long\n".to_vec();
        let expected_max_line_len = line2.len() - 1;
        let expected_line1 = std::str::from_utf8(&line1[0..10]).unwrap();
        debug!("expected_line1='{expected_line1}'");
        let expected_line2 = std::str::from_utf8(&line2[0..10]).unwrap();
        debug!("expected_line2='{expected_line2}'");
        let expected_line3 = std::str::from_utf8(&line3[0..10]).unwrap();
        debug!("expected_line3='{expected_line3}'");

        let data = [line1.clone(), line2.clone(), line3.clone()].concat();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 10);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line1));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), line1.len() - 1);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line2));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 2);
        assert_eq!(rtl.max_processed_full_line_len(), expected_max_line_len);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line3));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 3);
        assert_eq!(rtl.max_processed_full_line_len(), expected_max_line_len);
    }

    #[test]
    fn test_three_long_lines_3_is_longest() {
        let line1 = b"First line that's shorter but still too long\n".to_vec();
        let line2 = b"Second line that's shorter but still too long\n".to_vec();
        let line3 = b"Third line that is longest and longer than buffer\n".to_vec();
        let expected_max_line_len = line3.len() - 1;
        let expected_line1 = std::str::from_utf8(&line1[0..10]).unwrap();
        debug!("expected_line1='{expected_line1}'");
        let expected_line2 = std::str::from_utf8(&line2[0..10]).unwrap();
        debug!("expected_line2='{expected_line2}'");
        let expected_line3 = std::str::from_utf8(&line3[0..10]).unwrap();
        debug!("expected_line3='{expected_line3}'");

        let data = [line1.clone(), line2.clone(), line3.clone()].concat();
        let mut reader = Cursor::new(data);
        let mut rtl = ReadTruncatedLines::new(&mut reader, "test", 10);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line1));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 1);
        assert_eq!(rtl.max_processed_full_line_len(), line1.len() - 1);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line2));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 2);
        assert_eq!(rtl.max_processed_full_line_len(), line2.len() - 1);
        assert_eq!(rtl.read_truncated_line(), Some(expected_line3));
        assert_eq!(rtl.line_len(), 10);
        assert_eq!(rtl.line_number(), 3);
        assert_eq!(rtl.max_processed_full_line_len(), expected_max_line_len);
    }
}
