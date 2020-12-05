use std::io::{BufRead, Result as IOResult, ErrorKind::{Interrupted, InvalidData}, Error as IOError};

pub fn read_until_bytes<R: BufRead + ?Sized>(
    r: &mut R,
    delim: &[u8],
    buffer: &mut Vec<u8>,
) -> IOResult<(usize,bool)> { // (data_read,has_delim)
    debug_assert!(!delim.is_empty(),"Can't have an empty delimiter");
    let mut index = 0;
    let mut total_read = 0;
    loop {
        let available = match r.fill_buf() {
            Ok(n) => n,
            Err(ref e) if e.kind() == Interrupted => continue,
            Err(e) => return Err(e),
        };
        let len = available.len();
        if len == 0 { return Ok((total_read,false)); }
        let mut iter = available.iter().enumerate();
        let (read_count,is_done) = loop {
            match iter.next() { 
                Some((pos,byte)) => {
                    if !(index < delim.len()) { break (pos, true) }
                    if *byte == delim[index] { index += 1; } else { index = 0; }
                },
                None => break (len,false)
            };
        };
        buffer.extend_from_slice(&available[..read_count]);
        r.consume(read_count);
        total_read += read_count;
        if is_done { return Ok((total_read,true)) } // delimiter found
    };
}

pub struct SplitBufReadBy<'a,B> {
    pat: &'a[u8],
    buf: B,
}

impl<'a,B: BufRead> SplitBufReadBy<'a, B> { // be aware of \r \n bullshitery
    pub fn from_bufread(buf: B,pat: &'a [u8]) -> Self { Self { buf, pat } }
}

impl<B: BufRead> Iterator for SplitBufReadBy<'_,B> {
    type Item = IOResult<String>;

    fn next(&mut self) -> Option<IOResult<String>> {
        let mut buf = Vec::new();
        return match read_until_bytes(&mut self.buf, self.pat, &mut buf) {
            Ok((0,_)) => None, // if no data was read : No data left, leave
            Err(err) => Some(Err(err)),
            Ok((_len,needs_removal)) => {
                if needs_removal { buf.truncate(buf.len()-self.pat.len()); }
                Some(String::from_utf8(buf).map_err(|_from_uft8_error| IOError::new(InvalidData, "stream did not contain valid UTF-8")))
            }
        }
    }
}

#[cfg(tests)]
mod tests {
  use super::SplitBufReadBy;
	use std::io::BufReader;

	#[test] fn split_by_iter_test1() {
		let input: Vec<String> = SplitBufReadBy::from_bufread(BufReader::new("Hello-World-Hyphen".as_bytes()),"-".as_bytes()).map(|line| line.unwrap()).collect();
		assert_eq!(input,vec!["Hello","World","Hyphen"].iter().map(|str| str.to_string()).collect::<Vec<String>>())
	}

	#[test] fn split_by_iter_test2() {
		let input: Vec<String> = SplitBufReadBy::from_bufread(BufReader::new("Hello-+World-+Minus-+Plus".as_bytes()),"-+".as_bytes()).map(|line| line.unwrap()).collect();
		assert_eq!(input,vec!["Hello","World","Minus","Plus"].iter().map(|str| str.to_string()).collect::<Vec<String>>())
	}

	#[test] fn split_by_iter_test3() {
		let input: Vec<String> = SplitBufReadBy::from_bufread(BufReader::new("Hello-World--Hyphen-And-Newline--Last-match".as_bytes()),"--".as_bytes()).map(|line| line.unwrap()).collect();
		assert_eq!(input,vec!["Hello-World","Hyphen-And-Newline","Last-match"].iter().map(|str| str.to_string()).collect::<Vec<String>>())
	}
}
