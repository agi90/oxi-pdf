mod deflate;
mod bit_reader;
mod gzip;

use crate::gzip::rfc1952;
use crate::bit_reader::BitReader;

use std::io;
use std::io::{
    Write,
    Read,
    BufReader
};
use std::fs::File;

use std::env;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        eprintln!("ERROR: Too many arguments.");
        eprintln!("INFO: Args {:?}", args);
        eprintln!("USAGE: deflate-cmd filename.gz");
        eprintln!("USAGE: deflate-cmd < cat filename.gz");
        return Ok(());
    }

    let source: Box<Read> = if args.len() == 2 {
        Box::new(BufReader::new(File::open(args[1].clone())?))
    } else {
        Box::new(io::stdin())
    };

    let mut reader = BitReader::new(source);

    rfc1952(&mut reader, &mut io::stdout())?;

    Ok(())
}
