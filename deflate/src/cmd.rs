#![feature(reverse_bits)]

mod deflate;
mod bit_reader;
mod gzip;

use crate::gzip::rfc1952;
use crate::bit_reader::BitReader;

extern crate clap;
use clap::{Arg, App};

use std::io;
use std::io::{
    BufReader,
    BufWriter,
};

use std::fs::File;

fn main() -> io::Result<()> {
    let matches = App::new("Uncompress gzip archives.")
        .version("1.0")
        .author("Agi Sferro <agi@sferro.dev>")
        .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Output file, defaults to stdout")
                .takes_value(true))
        .arg(Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1))
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let output = matches.value_of("output");

    let source = Box::new(BufReader::new(File::open(input)?));
    let mut reader = BitReader::new(source);

    if let Some(file_name) = output {
        let mut result = Box::new(BufWriter::new(File::open(file_name)?));
        rfc1952(&mut reader, &mut result)?;
    } else {
        rfc1952(&mut reader, &mut io::stdout())?;
    }

    Ok(())
}
