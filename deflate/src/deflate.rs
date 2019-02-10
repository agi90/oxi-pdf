// This module implements deflate from RFC1951
// A copy of it is available at https://tools.ietf.org/html/rfc1951#section-1.1

use std::io;
use std::io::{
    Error,
    ErrorKind,
    Read,
};

use std::collections::HashMap;

use std::cmp;

use crate::bit_reader::{
    ReadBits,
    BitReader,
};

#[derive(Debug, PartialEq, Eq)]
enum EncodingType {
    NoCompression,
    FixedHuffman,
    DynamicHuffman,
}

impl EncodingType {
    fn from(data: u64) -> Option<EncodingType> {
        Some(match data {
            0b00 => EncodingType::NoCompression,
            0b10 => EncodingType::FixedHuffman,
            0b01 => EncodingType::DynamicHuffman,
            _ => return None,
        })
    }
}

pub fn rfc1950(data: &mut BitReader) -> io::Result<Vec<u8>> {
    let compression_method = data.read_number(4)?;
    let compression_info = data.read_number(4)?;
    let check_bits = data.read_number(5)?;
    let preset_dictionary = data.read_number(1)?;
    let compression_level = data.read_number(2)?;

    let checksum =
          ((compression_info as u16) << 12)
        + ((compression_method as u16) << 8)
        + ((compression_level as u16) << 6)
        + ((preset_dictionary as u16) << 5)
        +  (check_bits as u16);

    assert!(checksum % 31 == 0);
    assert!(compression_method == 8);

    if checksum % 31 != 0 || compression_method != 8 {
        // return Err(Error::new(ErrorKind::Other, "Header checksum doesn't mach."));
        panic!();
    }

    if preset_dictionary > 0 {
        // TODO: checksum
        let _adler32 = data.read_number(32)?;
    }

    rfc1951(data)
}

pub fn rfc1951(data: &mut BitReader) -> io::Result<Vec<u8>> {
    let mut decoded = vec![];
    loop {
        let bfinal = data.read_bits(1)?;
        let btype = data.read_bits(2)?;
        let fixed_literal_code = generate_fixed_huffman();
        let fixed_distance_code = generate_fixed_distance_code();

        match EncodingType::from(btype).unwrap() {
            EncodingType::NoCompression => {
                decoded.append(&mut read_no_compression(data)?);
            },
            EncodingType::FixedHuffman => {
                let adapter = HuffmanAdapter::new(data,
                    &fixed_literal_code, Some(&fixed_distance_code));
                read_huffman(adapter, &mut decoded)?;
            },
            EncodingType::DynamicHuffman => {
                let (literal_code, distance_code) =
                        read_huffman_code(data)?;
                let adapter = HuffmanAdapter::new(data, &literal_code,
                                                  Some(&distance_code));
                read_huffman(adapter, &mut decoded)?;
            },
        }

        if bfinal > 0 {
            break;
        }
    }

    // TODO: checksum

    Ok(decoded)
}

#[derive(Debug)]
struct HuffmanCode {
    codes: HashMap<usize, Vec<i64>>,
    min_length: usize,
    max_length: usize,
}

// Fixed distance codes are just 5-bit integers
fn generate_fixed_distance_code() -> HuffmanCode {
    let mut code_5_bits = vec![-1; 32];
    for i in 0 .. 32 {
        code_5_bits[i] = i as i64;
    }

    let mut codes = HashMap::new();
    codes.insert(5, code_5_bits);

    HuffmanCode {
        codes,
        min_length: 5,
        max_length: 5,
    }
}

// Fixed huffman table
//   Lit Value    Bits        Codes
//   ---------    ----        -----
//     0 - 143     8          00110000 through
//                            10111111
//   144 - 255     9          110010000 through
//                            111111111
//   256 - 279     7          0000000 through
//                            0010111
//   280 - 287     8          11000000 through
//                            11000111
fn generate_fixed_huffman() -> HuffmanCode {
    let mut mapped = 0;

    let mut code_8_bits = vec![-1; 256];
    for i in 0b00110000 ..= 0b10111111 {
        code_8_bits[i] = mapped;
        mapped += 1;
    }

    let mut code_9_bits = vec![-1; 512];
    for i in 0b110010000 ..= 0b111111111 {
        code_9_bits[i] = mapped;
        mapped += 1;
    }

    let mut code_7_bits = vec![-1; 128];
    for i in 0b0000000 ..= 0b0010111 {
        code_7_bits[i] = mapped;
        mapped += 1;
    }

    for i in 0b11000000 ..= 0b11000111 {
        code_8_bits[i] = mapped;
        mapped += 1;
    }

    let mut codes = HashMap::new();
    codes.insert(7, code_7_bits.to_vec());
    codes.insert(8, code_8_bits.to_vec());
    codes.insert(9, code_9_bits.to_vec());

    HuffmanCode {
        codes,
        min_length: 7,
        max_length: 9,
    }
}

// RFC1951 ~ 3.2.7
fn read_huffman_code(data: &mut BitReader)
        -> io::Result<(HuffmanCode, HuffmanCode)> {
    let hlit = data.read_number(5)? as usize + 257;
    let hdist = data.read_number(5)? as usize + 1;
    let hclen = data.read_number(4)? as usize + 4;

    let code_lengths = read_code_lengths(data, hclen)?;

    let codes = generate_codes(&code_lengths);

    let mut adapter = HuffmanAdapter::new(data, &codes, None);

    let literal_code_lengths = read_compressed_code_lengths(&mut adapter, hlit)?;
    let literal_codes = generate_codes(&literal_code_lengths[..]);

    let distance_code_lengths = read_compressed_code_lengths(&mut adapter, hdist)?;
    let distance_codes = generate_codes(&distance_code_lengths[..]);

    Ok((literal_codes, distance_codes))
}

const CODE_LENGTH_ORDER :[usize; 19] =
        [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

fn read_code_lengths(data: &mut BitReader, length: usize)
        -> io::Result<[u8; 19]> {
    let mut result = [0; 19];
    for i in 0..length {
        result[CODE_LENGTH_ORDER[i]] = data.read_number(3)? as u8;
    }
    Ok(result)
}

// RFC1951 ~ 3.2.7
fn read_compressed_code_lengths(data: &mut HuffmanAdapter, length: usize)
        -> io::Result<Vec<u8>> {
    let mut i = 0;
    let mut result = vec![0; length];
    let mut prev_code = 0;

    while i < length {
        let code = data.next_code()?;
        match code {
            0 ... 15 => {
                prev_code = code as u8;
                result[i] = prev_code;
                i += 1;
            },
            16 => {
                let length = data.read_number(2)? + 3;
                for _ in 0..length {
                    result[i] = prev_code;
                    i += 1;
                }
            },
            17 => {
                let length = data.read_number(3)? + 3;
                for _ in 0..length {
                    result[i] = 0;
                    i += 1;
                }
            },
            18 => {
                let length = data.read_number(7)? + 11;
                for _ in 0..length {
                    result[i] = 0;
                    i += 1;
                }
            },
            _ => {
                // return Err(Error::new(ErrorKind::Other, "Unknown Huffman Code"));
                panic!();
            }
        }
    }
    Ok(result)
}

// RFC1951 ~ 3.2.2
fn generate_codes(code_lengths: &[u8]) -> HuffmanCode {
    // Step 1
    let mut bl_count = vec![];
    let mut min_length = code_lengths.len();
    let mut max_length = 0;
    for x in code_lengths {
        let length = *x as usize;
        if length < min_length && length != 0 { min_length = length }
        if length > max_length { max_length = length }
        if bl_count.len() <= length {
            bl_count.resize(length + 1, 0);
        }

        // 0 lengths are unused codes
        if length != 0 {
            bl_count[length] += 1;
        }
    }

    // Step 2
    let mut next_code = vec![0; bl_count.len()];
    let mut code = 0;
    for bits in 1..bl_count.len() {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    // Step 3
    let mut codes: HashMap<usize, Vec<i64>> = HashMap::new();

    for n in 0..code_lengths.len() {
        let len = code_lengths[n] as usize;
        if len == 0 { continue; }

        codes.entry(len).or_insert(vec![-1; 1 << len]);
        codes.get_mut(&len).unwrap()[next_code[len]] = n as i64;

        next_code[len] += 1;
    }

    HuffmanCode { codes, min_length, max_length }
}

struct HuffmanAdapter<'a> {
    data: &'a mut BitReader,
    coder: &'a HuffmanCode,
    distance_coder: Option<&'a HuffmanCode>,
}

impl <'a> HuffmanAdapter<'a> {
    fn new(data: &'a mut BitReader, coder: &'a HuffmanCode,
           distance_coder: Option<&'a HuffmanCode>) -> HuffmanAdapter<'a> {
        HuffmanAdapter {
            data, coder, distance_coder
        }
    }

    fn read_number(&mut self, len: usize) -> io::Result<u64> {
        self.data.read_number(len)
    }

    fn next_code(&mut self) -> io::Result<u16> {
        self.next_code_impl(&self.coder)
    }

    fn next_distance(&mut self) -> io::Result<u16> {
        let distance_coder = &self.distance_coder
            .ok_or(Error::new(ErrorKind::Other,
                   "This Adapter does not have a distance coder."))?;
        self.next_code_impl(distance_coder)
    }

    fn next_code_impl(&mut self, coder: &HuffmanCode) -> io::Result<u16> {
        let mut x = self.data.read_bits(coder.min_length)? as usize;
        let mut length = coder.min_length;
        while length <= coder.max_length {
            if coder.codes.contains_key(&length) && coder.codes[&length][x] != -1 {
                return Ok(coder.codes[&length][x] as u16);
            } else {
                x = (x << 1) + self.data.read_bits(1)? as usize;
                length += 1;
            }
        }

        // return Err(Error::new(ErrorKind::Other, "Unknown Huffman Code"));
        panic!();
    }

    fn read_distance(&mut self, code: u16) -> io::Result<(usize, usize)> {
        //      Extra               Extra               Extra
        // Code Bits Length(s) Code Bits Lengths   Code Bits Length(s)
        // ---- ---- ------     ---- ---- -------   ---- ---- -------
        //  257   0     3       267   1   15,16     277   4   67-82
        //  258   0     4       268   1   17,18     278   4   83-98
        //  259   0     5       269   2   19-22     279   4   99-114
        //  260   0     6       270   2   23-26     280   4  115-130
        //  261   0     7       271   2   27-30     281   5  131-162
        //  262   0     8       272   2   31-34     282   5  163-194
        //  263   0     9       273   3   35-42     283   5  195-226
        //  264   0    10       274   3   43-50     284   5  227-257
        //  265   1  11,12      275   3   51-58     285   0    258
        //  266   1  13,14      276   3   59-66
        let (extra_bits, partial_length) = match code {
            257 => (0,  3),
            258 => (0,  4),
            259 => (0,  5),
            260 => (0,  6),
            261 => (0,  7),
            262 => (0,  8),
            263 => (0,  9),
            264 => (0, 10),
            265 => (1, 11),
            266 => (1, 13),
            267 => (1, 15),
            268 => (1, 17),
            269 => (2, 19),
            270 => (2, 23),
            271 => (2, 27),
            272 => (2, 31),
            273 => (3, 35),
            274 => (3, 43),
            275 => (3, 51),
            276 => (3, 59),
            277 => (4, 67),
            278 => (4, 83),
            279 => (4, 99),
            280 => (4, 115),
            281 => (5, 131),
            282 => (5, 163),
            283 => (5, 195),
            284 => (5, 227),
            285 => (0, 258),
            _ => return Err(Error::new(ErrorKind::Other, "Unexpected code length.")),
        };

        let length = partial_length + self.data.read_number(extra_bits)? as usize;

        let distance_code = self.next_distance()?;
        //      Extra           Extra               Extra
        // Code Bits Dist  Code Bits   Dist     Code Bits Distance
        // ---- ---- ----  ---- ----  ------    ---- ---- --------
        //   0   0    1     10   4     33-48    20    9   1025-1536
        //   1   0    2     11   4     49-64    21    9   1537-2048
        //   2   0    3     12   5     65-96    22   10   2049-3072
        //   3   0    4     13   5     97-128   23   10   3073-4096
        //   4   1   5,6    14   6    129-192   24   11   4097-6144
        //   5   1   7,8    15   6    193-256   25   11   6145-8192
        //   6   2   9-12   16   7    257-384   26   12  8193-12288
        //   7   2  13-16   17   7    385-512   27   12 12289-16384
        //   8   3  17-24   18   8    513-768   28   13 16385-24576
        //   9   3  25-32   19   8   769-1024   29   13 24577-32768
        let (extra_bits_distance, base_distance) = match distance_code {
           0 => (0, 1),
           1 => (0, 2),
           2 => (0, 3),
           3 => (0, 4),
           4 => (1, 5),
           5 => (1, 7),
           6 => (2, 9),
           7 => (2, 13),
           8 => (3, 17),
           9 => (3, 25),
           10 => (4, 33),
           11 => (4, 49),
           12 => (5, 65),
           13 => (5, 97),
           14 => (6, 129),
           15 => (6, 193),
           16 => (7, 257),
           17 => (7, 385),
           18 => (8, 513),
           19 => (8, 769),
           20 => (9, 1025),
           21 => (9, 1537),
           22 => (10, 2049),
           23 => (10, 3073),
           24 => (11, 4097),
           25 => (11, 6145),
           26 => (12, 8193),
           27 => (12, 12289),
           28 => (13, 16385),
           29 => (13, 24577),
           _ => return Err(Error::new(ErrorKind::Other, "Unexpected distance length.")),
        };

        let distance = base_distance
                + self.data.read_number(extra_bits_distance)? as usize;
        Ok((length, distance))
    }
}

fn read_huffman(mut data: HuffmanAdapter, out: &mut Vec<u8>) -> io::Result<()> {
    loop {
        let code = data.next_code();
        match code {
            Ok(x) => {
                if x < 256 {
                    out.push(x as u8);
                } else if x == 256 {
                    return Ok(());
                } else {
                    let (mut length, distance) = data.read_distance(x)?;
                    let start = out.len() - distance;

                    // If the buffer is not long enough, we will just repeat
                    // the characters until we fill the specified length
                    let end = cmp::min(out.len(), start + length);

                    let match_ = (&out[start..end]).to_vec();

                    loop {
                        // If this is the last repeated section we need to clip
                        // the match to make it fit in the buffer.
                        let bound = cmp::min(match_.len(), length);
                        out.append(&mut (&match_[0..bound]).to_vec());

                        if length > match_.len() {
                            length -= match_.len();
                        } else {
                            break;
                        }
                    }
                }
            },
            Err(error) => {
                match error.kind() {
                    _ => return Err(error),
                }
            },
        }
    }
}

fn read_no_compression(data: &mut BitReader) -> io::Result<Vec<u8>> {
    // Round to nearest byte
    data.read_remaining_byte()?;

    let len = data.read_number(16)? as u16;
    let check_len = !(data.read_number(16)? as u16);

    if len != check_len {
        // return Err(Error::new(ErrorKind::Other, "Length checksum doesn't mach."));
        panic!();
    }

    let mut data_buf = vec![0; len as usize];
    data.read_exact(&mut data_buf)?;

    Ok(data_buf)
}

#[cfg(test)]
mod test {
    use super::*;

    use std::io::{
        Cursor,
    };

    #[test]
    fn test_fixed_huffman_decode() {
        let data = vec![
            0x0B, 0x49, 0x2D, 0x2E, 0xC9, 0xCC, 0x4B, 0x0F, 0x81, 0x50, 0x00];

        let mut reader = BitReader::new(Box::new(Cursor::new(data)));

        let data = rfc1951(&mut reader).unwrap();
        assert_eq!(String::from_utf8(data).unwrap().as_str(),
            "TestingTesting");
    }
}
