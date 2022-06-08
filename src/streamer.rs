use std::io::{Read};

use anyhow::Result;
use nom::error::VerboseError;

use crate::json_parser::{root, JsonValue};

#[derive(Debug)]
pub struct Streamer<R> {
    buf: Vec<u8>,
    reader: R,
}

// is this an iterator?
impl<R> Streamer<R> {
    pub fn new(reader: R) -> Self {
	return Self {
	    buf: Vec::with_capacity(4096),
	    reader
	}
    }

    pub fn next_val(&mut self) -> Result<JsonValue> {
	let res = root::<VerboseError<&str>>(todo!());

	todo!()
    }
}
