use std::io;
use std::io::Read;

const REVERSE_TABLE: [u8; 256] = [
    0x00, 0x80, 0x40, 0xC0, 0x20, 0xA0, 0x60, 0xE0, 0x10, 0x90, 0x50, 0xD0,
    0x30, 0xB0, 0x70, 0xF0, 0x08, 0x88, 0x48, 0xC8, 0x28, 0xA8, 0x68, 0xE8,
    0x18, 0x98, 0x58, 0xD8, 0x38, 0xB8, 0x78, 0xF8, 0x04, 0x84, 0x44, 0xC4,
    0x24, 0xA4, 0x64, 0xE4, 0x14, 0x94, 0x54, 0xD4, 0x34, 0xB4, 0x74, 0xF4,
    0x0C, 0x8C, 0x4C, 0xCC, 0x2C, 0xAC, 0x6C, 0xEC, 0x1C, 0x9C, 0x5C, 0xDC,
    0x3C, 0xBC, 0x7C, 0xFC, 0x02, 0x82, 0x42, 0xC2, 0x22, 0xA2, 0x62, 0xE2,
    0x12, 0x92, 0x52, 0xD2, 0x32, 0xB2, 0x72, 0xF2, 0x0A, 0x8A, 0x4A, 0xCA,
    0x2A, 0xAA, 0x6A, 0xEA, 0x1A, 0x9A, 0x5A, 0xDA, 0x3A, 0xBA, 0x7A, 0xFA,
    0x06, 0x86, 0x46, 0xC6, 0x26, 0xA6, 0x66, 0xE6, 0x16, 0x96, 0x56, 0xD6,
    0x36, 0xB6, 0x76, 0xF6, 0x0E, 0x8E, 0x4E, 0xCE, 0x2E, 0xAE, 0x6E, 0xEE,
    0x1E, 0x9E, 0x5E, 0xDE, 0x3E, 0xBE, 0x7E, 0xFE, 0x01, 0x81, 0x41, 0xC1,
    0x21, 0xA1, 0x61, 0xE1, 0x11, 0x91, 0x51, 0xD1, 0x31, 0xB1, 0x71, 0xF1,
    0x09, 0x89, 0x49, 0xC9, 0x29, 0xA9, 0x69, 0xE9, 0x19, 0x99, 0x59, 0xD9,
    0x39, 0xB9, 0x79, 0xF9, 0x05, 0x85, 0x45, 0xC5, 0x25, 0xA5, 0x65, 0xE5,
    0x15, 0x95, 0x55, 0xD5, 0x35, 0xB5, 0x75, 0xF5, 0x0D, 0x8D, 0x4D, 0xCD,
    0x2D, 0xAD, 0x6D, 0xED, 0x1D, 0x9D, 0x5D, 0xDD, 0x3D, 0xBD, 0x7D, 0xFD,
    0x03, 0x83, 0x43, 0xC3, 0x23, 0xA3, 0x63, 0xE3, 0x13, 0x93, 0x53, 0xD3,
    0x33, 0xB3, 0x73, 0xF3, 0x0B, 0x8B, 0x4B, 0xCB, 0x2B, 0xAB, 0x6B, 0xEB,
    0x1B, 0x9B, 0x5B, 0xDB, 0x3B, 0xBB, 0x7B, 0xFB, 0x07, 0x87, 0x47, 0xC7,
    0x27, 0xA7, 0x67, 0xE7, 0x17, 0x97, 0x57, 0xD7, 0x37, 0xB7, 0x77, 0xF7,
    0x0F, 0x8F, 0x4F, 0xCF, 0x2F, 0xAF, 0x6F, 0xEF, 0x1F, 0x9F, 0x5F, 0xDF,
    0x3F, 0xBF, 0x7F, 0xFF,
];

// Waiting for u8::reverse_bits to be available in stable
fn reverse_bits(x: u8) -> u8 {
    REVERSE_TABLE[x as usize]
}

pub trait ReadBits {
    fn read_bits(&mut self, len: usize) -> io::Result<u64>;
    fn read_remaining_byte(&mut self) -> io::Result<u8>;
    fn read_number(&mut self, len: usize) -> io::Result<u64>;
}

pub struct BitReader {
    data: Box<Read>,
    buffer: u8,
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

    fn read_from_single_byte(&mut self, len: usize) -> io::Result<u8> {
        assert!(len <= 8);

        if len == 0 { return Ok(0); }
        let mut buf = [0; 1];
        if self.buffer_size < len {
            self.data.read_exact(&mut buf)?;
            let new_byte = reverse_bits(buf[0]);
            let extra_bits = len - self.buffer_size;
            if extra_bits == 8 {
                buf[0] = new_byte;
                self.buffer = 0;
            } else {
                buf[0] = (self.buffer << extra_bits) +
                    ((new_byte & (U8_BIT_MASK << (8 - extra_bits))) >>
                        (8 - extra_bits));
                self.buffer = new_byte & (U8_BIT_MASK >> extra_bits);
            }
            self.buffer_size = 8 - extra_bits;
        } else {
            let mask = (U8_BIT_MASK >> 8 - len) << (self.buffer_size - len);
            buf[0] = (self.buffer & mask) >> (self.buffer_size - len);
            self.buffer = self.buffer & !mask;
            self.buffer_size -= len;
        }

        Ok(buf[0])
    }
}

const U8_BIT_MASK: u8 = 0b1111_1111;

impl Read for BitReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.data.read(buf)
    }
}

impl ReadBits for BitReader {
    fn read_remaining_byte(&mut self) -> io::Result<u8> {
        Ok(self.read_bits(self.buffer_size)? as u8)
    }

    fn read_bits(&mut self, mut len: usize) -> io::Result<u64> {
        assert!(len <= 64);

        let mut buf = 0u64;

        loop {
            let to_read = if len > 8 { 8 } else { len };
            let single_byte = self.read_from_single_byte(to_read)? as u64;
            buf = (buf << to_read) + single_byte;

            if len > 8 {
                len -= 8;
            } else {
                break;
            }
        }

        Ok(buf)
    }

    fn read_number(&mut self, mut len: usize) -> io::Result<u64> {
        assert!(len <= 64);

        if len == 0 { return Ok(0); }

        let mut buf = 0u64;
        let mut shift = 0;

        loop {
            let to_read = if len > 8 { 8 } else { len };
            let single_byte = self.read_from_single_byte(to_read)?;

            let reversed: u8 = reverse_bits(single_byte << (8 - to_read));
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

        assert_eq!(reader.read_bits(8).unwrap() as u8, reverse_bits(0x78u8));
        assert_eq!(reader.read_bits(8).unwrap() as u8, reverse_bits(0x9Cu8));
        assert_eq!(reader.read_bits(8).unwrap() as u8, reverse_bits(0x6Bu8));
    }

    #[test]
    fn test_read_number_long() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0x78, 0x9C, 0x6B])));

        let actual = reader.read_number(24).unwrap() as u32;
        assert_eq!(actual, 0x6B9C78);
    }

    #[test]
    fn test_read_bits_really_long() {
        let mut reader = BitReader::new(Box::new(Cursor::new(vec![
            0x78, 0x9C, 0x6B])));

        let expected =
              ((reverse_bits(0x78u8) as u32) << 16)
            + ((reverse_bits(0x9Cu8) as u32) << 8)
            +   reverse_bits(0x6Bu8) as u32;

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
