use std::io::Read;
// use std::collections::VecDeque;

use anyhow::Result;
use nom::{error::VerboseError, Finish};
use tracing::debug;

use crate::json_parser::{root, JsonValue};

#[derive(Debug)]
pub struct Streamer<R> {
    buf: Vec<u8>,
    start: usize,
    end: usize,
    reader: R,
    eof: bool,
}

const DEFAULT_BUF_SIZE: usize = 100;
const ZEROS: [u8; DEFAULT_BUF_SIZE] = [0u8; DEFAULT_BUF_SIZE];

// is this an iterator?
impl<R: Read> Streamer<R> {
    pub fn new(reader: R) -> Self {
        let mut buf = Vec::with_capacity(DEFAULT_BUF_SIZE);
        buf.extend_from_slice(&ZEROS);
        Self {
            buf,
            reader,
            start: 0,
            end: 0,
            eof: false,
        }
    }

    // returns bytes consumed. 0 -> EOF
    fn consume(&mut self) -> Result<usize> {
        debug!(
            "consume - buflen={}; cap={}, start={}, end={}",
            self.buf.len(),
            self.buf.capacity(),
            self.start,
            self.end
        );

        if self.end >= self.buf.len() {
            self.grow_buf();
        }
        let b = &mut self.buf[self.end..];

        debug!("{}", b.len());

        let n = self.reader.read(b)?;

        self.end += n;

        debug!(
            "consumed; buf={:?}",
            "<".to_string()
                + std::str::from_utf8(&self.buf[0..self.start]).unwrap()
                + "> | <"
                + std::str::from_utf8(self.buf()).unwrap()
                + "> | <"
                + std::str::from_utf8(&self.buf[self.end..]).unwrap()
                + ">"
        );

        Ok(n)
    }

    fn buf(&self) -> &[u8] {
        &self.buf[self.start..self.end]
    }

    // probably can be better, with a vecdeque maybe, idk
    fn realign_buf(&mut self) {
        let mut new = Vec::with_capacity(DEFAULT_BUF_SIZE);
        new.extend_from_slice(self.buf());
        self.buf = new;
    }

    fn grow_buf(&mut self) {
        // TODO: if buf is too big and start is big too, realign_buf here
        // for now, the buf keeps growing on consume
        self.buf.reserve(DEFAULT_BUF_SIZE);
        self.buf.extend_from_slice(&ZEROS);
    }

    fn advance_by(&mut self, n: usize) {
        let len = self.end - self.start;

        self.start += n;
        if self.start >= self.end {
            self.realign_buf();
            self.start = 0;
            self.end = len;
        }
    }
}

impl<R: Read> Iterator for Streamer<R> {
    type Item = Result<JsonValue>;

    fn next(&mut self) -> Option<Self::Item> {
        let input = self.buf();
        let input_len = input.len();
        debug!("{:?}", std::str::from_utf8(input).unwrap());

        let res = root::<VerboseError<&[u8]>>(input);

        match res {
            Ok((remaining, val)) => {
                debug!("{:?}", &std::str::from_utf8(remaining));
                debug!("{:?}", &val);
                // is remaining always at the end? i think so
                let remaining_len = remaining.len();
                self.advance_by(input_len - remaining_len);
                Some(Ok(val))
            }
            Err(ref e) if e.is_incomplete() => {
                debug!("{:?}", &e);

                if self.eof {
                    debug!("done");
                    return None;
                }

                match self.consume() {
                    Ok(0) => {
                        self.eof = true;
                        debug!("got eof but still trying again");
                        // try again. TODO: better
                        self.next()
                    }
                    Err(e) => {
                        debug!("consume err");
                        Some(Err(e))
                    }
                    Ok(_) => {
                        debug!("trying again");
                        // try again. TODO: better
                        self.next()
                    }
                }
            }
            Err(_) => {
                let err = res.finish().unwrap_err();
                dbg!(err
                    .errors
                    .iter()
                    .map(|(data, kind)| (std::str::from_utf8(data).unwrap(), kind))
                    .collect::<Vec<_>>());

                // dbg!(nom::error::convert_error(
                //     std::str::from_utf8(input).unwrap(),
                //     err
                // ));
                return Some(Err(anyhow::anyhow!("idk")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_consumes() -> Result<()> {
        let mut r = r#"{"hello": 42}"#.as_bytes();
        let mut streamer = Streamer::new(&mut r);
        dbg!(&streamer.buf.len());
        let n = streamer.consume()?;
        assert_ne!(n, 0);

        Ok(())
    }

    #[test]
    fn it_works() -> Result<()> {
        let mut r = r#"
{"hello": 42}
{"bye": 37}
["wat", 101]
"#
        .as_bytes();
        let mut streamer = Streamer::new(&mut r);

        let v = streamer.next().unwrap()?;
        assert_eq!(v["hello"], JsonValue::Num(42.0));

        let v = streamer.next().unwrap()?;
        assert_eq!(v["bye"], JsonValue::Num(37.0));

        let v = streamer.next().unwrap()?;
        assert_eq!(v[0], JsonValue::Str("wat".to_string()));
        assert_eq!(v[1], JsonValue::Num(101.0));

        // TODO: it always fails to parse the last thing. why??

        assert!(streamer.next().is_none());

        Ok(())
    }
}
