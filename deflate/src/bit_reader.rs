use std::io;
use std::io::{
    Read,
    Error,
    ErrorKind,
};

pub trait ReadBits {
    fn read_bits(&mut self, len: usize) -> io::Result<u64>;
    fn read_remaining_byte(&mut self) -> io::Result<u8>;
    fn read_number(&mut self, len: usize) -> io::Result<u64>;
}

pub struct BitReader {
    data: Box<Read>,
    buffer: u64,
    buffer_size: usize,
}

impl BitReader {
    pub fn new(data: Box<Read>) -> BitReader {
        BitReader {
            data,
            buffer: 0,
            buffer_size: 0,
        }
    }

}

const U64_BIT_MASK: u64 = 0xFFFFFFFFFFFFFFFF;

impl Read for BitReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.buffer_size % 8 != 0 {
            // hairy situation, let's crash for now
            panic!("Misaligned buffer size.");
        }

        // Let's collect the remaining buffer first
        let mut i = 0;
        while self.buffer_size > 0 && i < buf.len() {
            buf[i] = self.read_number(8)? as u8;
            i += 1;
        }

        // When the buffer is exhausted, let's read from the raw data
        self.data.read(&mut buf[i..]).map(|total| total + i)
    }
}

impl ReadBits for BitReader {
    fn read_remaining_byte(&mut self) -> io::Result<u8> {
        Ok(self.read_bits(self.buffer_size % 8)? as u8)
    }

    fn read_bits(&mut self, mut len: usize) -> io::Result<u64> {
        assert!(len <= 64);

        let mut start = 0;
        let mut result = 0;

        if self.buffer_size < len {
            result = self.buffer;
            start = self.buffer_size;

            let mut buf = [0; 8];
            let read_len = self.data.read(&mut buf)?;

            self.buffer = u64::from_le_bytes(buf);

            len -= self.buffer_size;
            self.buffer_size = read_len * 8;
        }

        // If we still don't have enough bits there's nothing we can do
        if self.buffer_size < len {
            return Err(Error::new(ErrorKind::UnexpectedEof,
                "Unexpected code length."));
        }

        // Now let's combine the previous buffer and the current buffer and invert.
        // e.g.
        // result = 00000000000000000000000000000000000000000XXXXXXXXXXXXXXX
        //                                                   ^             ^
        //                                                   ---------------
        //                                                        start
        //
        // piece  = 00000000000000000000000000000000YYYYYYYYY000000000000000
        //                                          ^       ^
        //                                          ---------
        //                                             len
        //
        // out    = 00000000000000000000000000000000XXXXXXXXXXXXXXXYYYYYYYYY

        let mut piece = (self.buffer & (U64_BIT_MASK >> 64 - len)) << start;
        result = ((piece + result) << (64 - len - start)).reverse_bits();

        self.buffer = (self.buffer >> len);
        self.buffer_size -= len;

        Ok(result)
    }

    fn read_number(&mut self, mut len: usize) -> io::Result<u64> {
        assert!(len <= 64);

        if len == 0 { return Ok(0); }

        let mut buf = 0u64;
        let mut shift = 0;

        loop {
            let to_read = if len > 8 { 8 } else { len };
            let single_byte = self.read_bits(to_read)? as u8;

            let reversed: u8 = (single_byte << (8 - to_read)).reverse_bits();
            buf += (reversed as u64) << shift;

            if len > 8 {
                len -= 8;
                shift += 8;
            } else {
                break;
            }
        }

        Ok(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::io::{
        Cursor,
    };

    fn test_bits(data: u8, len: usize, expected: u8) -> BitReader {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![data])));
        let actual = reader.read_bits(len).unwrap();

        assert_eq!(actual as u8, expected);

        reader
    }

    #[test]
    fn test_read_bits() {
        test_bits(0b11111111, 2, 0b00000011);
        test_bits(0b11111111, 3, 0b00000111);
        test_bits(0b11111111, 4, 0b00001111);
        test_bits(0b11111111, 7, 0b01111111);
        test_bits(0b11001101, 8, 0b10110011);
        test_bits(0b11001101, 7, 0b01011001);
        test_bits(0b11111111, 8, 0b11111111);
        test_bits(0b00000001, 2, 0b00000010);
    }

    #[test]
    fn test_read_bits_long() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0x78, 0x9C, 0x6B])));

        assert_eq!(reader.read_bits(8).unwrap() as u8, 0x78u8.reverse_bits());
        assert_eq!(reader.read_bits(8).unwrap() as u8, 0x9Cu8.reverse_bits());
        assert_eq!(reader.read_bits(8).unwrap() as u8, 0x6Bu8.reverse_bits());
    }

    #[test]
    fn test_read_number_long() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0x78, 0x9C, 0x6B])));

        let actual = reader.read_number(24).unwrap() as u32;
        assert_eq!(actual, 0x6B9C78);
    }

    #[test]
    fn test_read_number_very_long_chain() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xFF, 0xAB])));

        assert_eq!(reader.read_number(8).unwrap(), 0x12);
        assert_eq!(reader.read_number(8).unwrap(), 0x34);
        assert_eq!(reader.read_number(8).unwrap(), 0x56);
        assert_eq!(reader.read_number(8).unwrap(), 0x78);
        assert_eq!(reader.read_number(8).unwrap(), 0x9A);
        assert_eq!(reader.read_number(8).unwrap(), 0xBC);
        assert_eq!(reader.read_number(8).unwrap(), 0xDE);
        assert_eq!(reader.read_number(8).unwrap(), 0xFF);
        assert_eq!(reader.read_number(8).unwrap(), 0xAB);
    }

    #[test]
    fn test_read_number_very_long() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0xFF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xFF])));

        assert_eq!(reader.read_number(8).unwrap(), 0xFF);
        assert_eq!(reader.read_number(64).unwrap(), 0xFFDEBC9A78563412);
    }

    #[test]
    fn test_read_bits_really_long() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0x78, 0x9C, 0x6B])));

        let expected =
              ((0x78u8.reverse_bits() as u32) << 16)
            + ((0x9Cu8.reverse_bits() as u32) << 8)
            +   0x6Bu8.reverse_bits() as u32;

        let actual = reader.read_bits(24).unwrap() as u32;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_fixed_huffman_bits() {
        let data = vec![
            0x0B, 0x49, 0x2D, 0x2E, 0xC9, 0xCC, 0x4B, 0x0F, 0x81, 0x50, 0x00];

        let mut reader = BitReader::new(Box::new(Cursor::new(data)));

        assert_eq!(reader.read_bits(1).unwrap(), 0b1);
        assert_eq!(reader.read_bits(2).unwrap(), 0b10);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10000100);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10010101);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10100011);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10100100);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10011001);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10011110);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10010111);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10000100);
        assert_eq!(reader.read_bits(7).unwrap(), 0b0000100);
        assert_eq!(reader.read_bits(5).unwrap(), 0b00101);
        assert_eq!(reader.read_bits(1).unwrap(), 0b0);
        assert_eq!(reader.read_bits(7).unwrap(), 0b0000000);
    }

    #[test]
    fn test_read_remaining_byte() {
        let data = vec![
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x4B, 0x0F, 0x81, 0x50, 0x00];

        let mut reader = BitReader::new(Box::new(Cursor::new(data)));

        assert_eq!(reader.read_bits(1).unwrap(), 0b1);
        assert_eq!(reader.read_bits(3).unwrap(), 0b111);
        assert_eq!(reader.read_remaining_byte().unwrap(), 0b1111);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1111);
    }

    #[test]
    fn test_read_bits_continuation() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0b11111111, 0b10001111])));

        let mut actual = reader.read_bits(2).unwrap();
        assert_eq!(actual, 0b00000011);

        actual = reader.read_bits(4).unwrap();
        assert_eq!(actual, 0b00001111);

        actual = reader.read_bits(4).unwrap();
        assert_eq!(actual, 0b00001111);

        actual = reader.read_bits(6).unwrap();
        assert_eq!(actual, 0b00110001);

        assert!(reader.read_bits(1).is_err())
    }
}
