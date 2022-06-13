use std::io::{BufRead, Result as IOResult, ErrorKind::{Interrupted, InvalidData}, Error as IOError};

pub trait SplitByBytes: BufRead {

    fn read_until_bytes (
        &mut self,
        delim: &[u8],
        buffer: &mut Vec<u8>,
    ) -> IOResult<(usize, usize)> { // (data_read, delim_bytes_tail_size)
        assert!(!delim.is_empty(), "Cannot have an empty delimiter");
        let mut index = 0;
        let mut total_read = 0;
        loop {
            let available = match self.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == Interrupted => continue,
                Err(e) => return Err(e)
            };
            if available.is_empty() { return Ok((total_read, index)); }
            let mut iter = available.iter();
            let mut sub_read = 0;
            let (read_count, is_done) = loop {
                match iter.next() {
                    Some(byte) => {
                        sub_read += 1;
                        if *byte == delim[index] {
                            index += 1;
                            if index == delim.len() { break (sub_read, true) }
                        } else { index = 0; }
                    },
                    None => break (sub_read, false)
                };
            };
            buffer.extend_from_slice(&available[..read_count]);
            self.consume(read_count);
            total_read += read_count;
            if is_done { return Ok((total_read, index)) }
        }
    }
    
    fn split_by_bytes(self, pattern: &[u8]) -> SplitBufReadByBytesIter<'_, Self> where Self: Sized { SplitBufReadByBytesIter { buf: self, pattern } }

}

impl<B: BufRead> SplitByBytes for B { /* */ }

pub struct SplitBufReadByBytesIter<'a, B: BufRead> {
    pattern : &'a[u8],
    buf: B
}

impl<B: BufRead> Iterator for SplitBufReadByBytesIter<'_,B> {
    type Item = IOResult<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::new();
        match self.buf.read_until_bytes(self.pattern, &mut buf) {
            Ok((0, _)) => None,
            Err(err) => Some(Err(err)),
            Ok((_len, pattern_tail)) => {
                if pattern_tail == self.pattern.len() { buf.truncate(buf.len() - self.pattern.len()); }
                Some(String::from_utf8(buf).map_err(|_from_uft8_error| IOError::new(InvalidData, "stream did not contain valid UTF-8")))
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::SplitByBytes;
    use std::io::BufReader;

    #[test] pub fn split_by_iter_test1() {
    let input: Vec<String> = BufReader::new("Hello-World-Hyphen".as_bytes()).split_by_bytes(b"-").map(|line| line.unwrap()).collect();
    	assert_eq!(input, vec!["Hello", "World", "Hyphen"])
    }

    #[test] pub fn split_by_iter_test2() {
    let input: Vec<String> = BufReader::new("Hello-+World-+Minus-+Plus".as_bytes()).split_by_bytes(b"-+").map(|line| line.unwrap()).collect();
    	assert_eq!(input, vec!["Hello", "World", "Minus", "Plus"])
    }

    #[test] pub fn split_by_iter_test3() {
    	let input: Vec<String> = BufReader::new("Hello------World---Hyphen--And-Newline-----Last-match".as_bytes()).split_by_bytes(b"---").map(|line| line.unwrap()).collect();
    	assert_eq!(input, vec!["Hello", "", "World", "Hyphen--And-Newline", "--Last-match"])
    }
}
