extern crate deflate;

use std::fs::File;
use std::io::Read;

use deflate::{
    BitReader,
    rfc1952,
};

#[test]
fn test_rfc1952() {
    let file = File::open("tests/data.gz").unwrap();
    let mut reader = BitReader::new(Box::new(file));
    let decompressed = rfc1952(&mut reader).unwrap();

    let mut expected_file = File::open("tests/expected.txt").unwrap();
    let mut expected = vec![];
    expected_file.read_to_end(&mut expected);

    assert_eq!(decompressed, expected);
}
