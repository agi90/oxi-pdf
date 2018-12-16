mod parser;

use std::fs::File;
use std::io::Read;

use crate::parser::parse_pdf;

fn main() -> std::io::Result<()> {
    let mut file = File::open("pdf.pdf")?;
    let mut contents = vec![];
    file.read_to_end(&mut contents)?;

    let pdf = parse_pdf(&contents[..]).unwrap();
    println!("{:?}", pdf);

    Ok(())
}
