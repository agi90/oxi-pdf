#[macro_use]
mod parser;
mod resolver;
mod types;
mod font;

use std::fs::File;
use std::io::Read;

use crate::parser::parse_pdf;
use crate::resolver::resolve_pdf;

fn main() -> std::io::Result<()> {
    let mut file = File::open("pdf.pdf")?;
    let mut contents = vec![];
    file.read_to_end(&mut contents)?;

    let pdf = parse_pdf(&contents[..]).unwrap();
    resolve_pdf(&pdf).unwrap();

    Ok(())
}
