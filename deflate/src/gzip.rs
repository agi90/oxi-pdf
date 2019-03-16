// This module implements gzip from RFC1952
// A copy of it is available at https://www.ietf.org/rfc/rfc1952.txt

use std::io;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Write,
};

use std::collections::HashSet;

use crate::bit_reader::{
    BitReader,
    ReadBits,
};

use crate::deflate::rfc1951;

#[derive(Debug, PartialEq, Eq, Hash)]
// 2.3.1
enum Flag {
    Text,
    Hcrc,
    Extra,
    Name,
    Comment,
}

impl Flag {
    fn from(data: u8) -> HashSet<Flag> {
        let mut result = HashSet::new();

        if data & 0b0000_0001 > 0 { result.insert(Flag::Text); }
        if data & 0b0000_0010 > 0 { result.insert(Flag::Hcrc); }
        if data & 0b0000_0100 > 0 { result.insert(Flag::Extra); }
        if data & 0b0000_1000 > 0 { result.insert(Flag::Name); }
        if data & 0b0001_0000 > 0 { result.insert(Flag::Comment); }

        result
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
// 2.3.1
enum Os {
    FatFilesystem,
    Amiga,
    Vms,
    Unix,
    VmCms,
    AtariTos,
    HpfsFilesystem,
    Macintosh,
    ZSystem,
    CpM,
    Tops20,
    NtfsFilesystem,
    Qdos,
    AcornRiscos,
    Unknown,
}

impl Os {
    fn from(data: u8) -> Option<Os> {
        Some(match data {
            0   => Os::FatFilesystem,
            1   => Os::Amiga,
            2   => Os::Vms,
            3   => Os::Unix,
            4   => Os::VmCms,
            5   => Os::AtariTos,
            6   => Os::HpfsFilesystem,
            7   => Os::Macintosh,
            8   => Os::ZSystem,
            9   => Os::CpM,
            10  => Os::Tops20,
            11  => Os::NtfsFilesystem,
            12  => Os::Qdos,
            13  => Os::AcornRiscos,
            255 => Os::Unknown,
            _ => { return None; },
        })
    }
}

// 2.3
pub fn rfc1952(data: &mut BitReader, out: &mut Write) -> io::Result<usize> {
    if data.read_number(16)? != 0x8B1F {
        return Err(Error::new(ErrorKind::Other, "Missing gzip magic number"));
    }

    if data.read_number(8)? != 0x08 {
        // 0x08 is DEFLATE RFC1951, which is the only compression method we
        // implement.
        return Err(Error::new(ErrorKind::Other, "Unknown compression method."));
    }

    let flags = Flag::from(data.read_number(8)? as u8);
    let _time = data.read_number(32)?;
    let _xfl = data.read_number(8)?;

    let _os = Os::from(data.read_number(8)? as u8)
        .ok_or(Error::new(ErrorKind::Other, "Unknown OS"))?;

    if flags.contains(&Flag::Extra) {
        // TODO:
        unimplemented!();
    }

    let _name;
    if flags.contains(&Flag::Name) {
        _name = read_name(data)?;
    } else {
        _name = "unknown".to_string();
    }

    if flags.contains(&Flag::Comment) {
        // TODO:
        unimplemented!();
    }

    if flags.contains(&Flag::Hcrc) {
        // TODO:
        unimplemented!();
    }

    let decompressed_size = rfc1951(data, out)?;

    data.read_remaining_byte()?;

    // TODO: checksum
    let _crc32 = data.read_number(32)?;
    let size = data.read_number(32)?;

    if decompressed_size != size as usize {
        return Err(Error::new(ErrorKind::Other, "Input size does not match."));
    }

    Ok(decompressed_size)
}

pub fn read_name(data: &mut BitReader) -> io::Result<String> {
    let mut name_bytes = vec![];
    let mut buf = [0xFF];
    while buf[0] != 0x00 {
        data.read_exact(&mut buf)?;
        name_bytes.push(buf[0]);
    }
    Ok(String::from_utf8(name_bytes)
        .unwrap_or("UnparsableName".to_string()))
}
