use std::{
    cmp,
    collections::HashMap,
    str,
    str::FromStr,
    io::{
        Cursor,
    },
    mem,
    convert::From,
    ops::Try,
};

use crate::deflate::{
    BitReader,
    rfc1950,
};

const ASCII_NUL: u8                  = 0x00;
const ASCII_BACKSPACE: u8            = 0x08;
const ASCII_HORIZONTAL_TAB: u8       = 0x09;
const ASCII_LINE_FEED: u8            = 0x0A;
const ASCII_FORM_FEED: u8            = 0x0C;
const ASCII_CARRIAGE_RETURN: u8      = 0x0D;
const ASCII_SPACE: u8                = 0x20;
const ASCII_EXCLAMATION_MARK: u8     = 0x21;
const ASCII_NUMBER_SIGN: u8          = 0x23;
const ASCII_PERCENT_SIGN: u8         = 0x25;
const ASCII_LEFT_PARENTHESIS: u8     = 0x28;
const ASCII_RIGHT_PARENTHESIS: u8    = 0x29;
const ASCII_SOLIDUS: u8              = 0x2F;
const ASCII_ZERO: u8                 = 0x30;
const ASCII_ONE: u8                  = 0x31;
const ASCII_TWO: u8                  = 0x32;
const ASCII_THREE: u8                = 0x33;
const ASCII_FOUR: u8                 = 0x34;
const ASCII_FIVE: u8                 = 0x35;
const ASCII_SIX: u8                  = 0x36;
const ASCII_SEVEN: u8                = 0x37;
const ASCII_EIGHT: u8                = 0x38;
const ASCII_NINE: u8                 = 0x39;
const ASCII_LESS_THAN_SIGN: u8       = 0x3C;
const ASCII_GREATER_THAN_SIGN: u8    = 0x3E;
const ASCII_A: u8                    = 0x41;
const ASCII_B: u8                    = 0x42;
const ASCII_C: u8                    = 0x43;
const ASCII_D: u8                    = 0x44;
const ASCII_E: u8                    = 0x45;
const ASCII_F: u8                    = 0x46;
const ASCII_R: u8                    = 0x52;
const ASCII_LEFT_SQUARE_BRACKET: u8  = 0x5B;
const ASCII_REVERSE_SOLIDUS: u8      = 0x5C;
const ASCII_RIGHT_SQUARE_BRACKET: u8 = 0x5D;
const ASCII_A_LOWERCASE: u8          = 0x61;
const ASCII_B_LOWERCASE: u8          = 0x62;
const ASCII_C_LOWERCASE: u8          = 0x63;
const ASCII_D_LOWERCASE: u8          = 0x64;
const ASCII_E_LOWERCASE: u8          = 0x65;
const ASCII_F_LOWERCASE: u8          = 0x66;
const ASCII_N_LOWERCASE: u8          = 0x6E;
const ASCII_TILDE: u8                = 0x7E;

fn resolve_dictionary<F>(dictionary: PdfDictionary, resolve: &mut F) -> PdfDictionary
where F: FnMut(&Key) -> PdfObject {
    let mut result = HashMap::new();

    for (key, object) in dictionary.data {
        match object {
            PdfObject::Reference(r) => {
                result.insert(key, resolve(&r).clone());
            },
            x => { result.insert(key, x); },
        }
    }

    PdfDictionary::new(result)
}

macro_rules! block {
    ($data: ident, $f: ident) => {
        {
            let result;
            if let Res::Found(r) = $f($data) {
                $data = r.remaining;
                result = r.data;
            } else {
                return Res::NotFound;
            }

            result
        }
    };
    ($data: expr, $f: ident, $param: expr) => {
        {
            let result;
            if let Res::Found(r) = $f($data, $param) {
                $data = r.remaining;
                result = r.data;
            } else {
                return Res::NotFound;
            }

            result
        }
    }
}

macro_rules! optional {
    ($data: ident, $f: ident) => {
        {
            let mut result = None;
            if let Res::Found(r) = $f($data) {
                $data = r.remaining;
                result = Some(r.data);
            }

            result
        }
    };
}

macro_rules! repeat {
    ($data: ident, $f: ident) => {
        {
            let result;
            if let Res::Found(r) = $f($data) {
                $data = r.remaining;
                result = r.data;
            } else {
                break;
            }

            result
        }
    };
}

macro_rules! exact {
    ($data: ident, $str: expr) => {
        if let Res::Found(r) = exact($data, $str) {
            $data = r.remaining;
        } else {
            return Res::NotFound;
        }
    }
}

macro_rules! requires {
    ($data: ident, $f: ident) => {
        if $data.len() == 0 || !$f($data[0]) {
            return Res::NotFound;
        }
    }
}

macro_rules! ascii {
    ($data: ident, $f: ident) => {
        if $data.len() == 0 || $data[0] != $f {
            return Res::NotFound;
        } else {
            $data = &$data[1..];
        }
    }
}

// 7.2.2
fn is_whitespace(data: u8) -> bool {
    match data {
        ASCII_NUL
        | ASCII_HORIZONTAL_TAB
        | ASCII_LINE_FEED
        | ASCII_FORM_FEED
        | ASCII_CARRIAGE_RETURN
        | ASCII_SPACE => true,
        _ => false
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Res<'a, T> {
    Found(Found<'a, T>),
    NotFound,
    Error,
}

#[derive(Debug, PartialEq, Eq)]
struct Found<'a, T> {
    data: T,
    remaining: &'a [u8],
}

impl <'a, T> Try for Res<'a, T> {
    type Ok = Res<'a, T>;
    type Error = Res<'a, T>;

    fn into_result(self) -> Result<Res<'a, T>, Res<'a, T>> {
        match self {
            Res::Found(x) => Ok(Res::Found(x)),
            Res::Error => Err(Res::Error),
            Res::NotFound => Err(Res::Error),
        }
    }

    fn from_error(v: Res<'a, T>) -> Self {
        v
    }

    fn from_ok(v: Res<'a, T>) -> Self {
        v
    }
}

impl <'a, T> From<String> for Res<'a, T> {
    fn from(_: String) -> Self {
        Res::Error
    }
}

impl <'a, T> Res<'a, T> {
    fn found(data: T, remaining: &[u8]) -> Res<T> {
        Res::Found(Found {
            data: data,
            remaining: remaining,
        })
    }

    fn is_found(&self) -> bool {
        match self {
            Res::Found(_) => true,
            _ => false,
        }
    }

    fn map<U, F> (self, mapper: F) -> Res<'a, U>
    where F: FnOnce(T) -> Option<U> {
        match self {
            Res::Found(Found{ data, remaining }) =>
                mapper(data).map(|d| Res::found(d, remaining))
                    .unwrap_or(Res::NotFound),
            Res::NotFound => Res::NotFound,
            Res::Error => Res::Error,
        }
    }
}

impl <'a> Res<'a, i64> {
    fn i64(data: &[u8], remaining: &'a [u8]) -> Res<'a, i64> {
        Res::string(data.to_vec(), remaining)
            .map(|s| i64::from_str(&s).ok())
    }
}

impl <'a> Res<'a, f64> {
    fn f64(data: &[u8], remaining: &'a [u8]) -> Res<'a, f64> {
        Res::string(data.to_vec(), remaining)
            .map(|s| f64::from_str(&s).ok())
    }
}

impl <'a> Res<'a, String> {
    fn string(data: Vec<u8>, remaining: &'a [u8]) -> Res<'a, String> {
        String::from_utf8(data)
            .map(|s| Res::found(s, remaining))
            .unwrap_or(Res::found("ERROR: Unparsable string.".to_string(), remaining))
    }
}

fn eol(data: &[u8]) -> Res<'_, ()> {
    if data.len() == 0 {
        return Res::NotFound;
    }

    match data[0] {
        ASCII_LINE_FEED => Res::found((), &data[1..]),
        ASCII_CARRIAGE_RETURN =>
            if data.len() > 1 && data[1] == ASCII_LINE_FEED {
                Res::found((), &data[2..])
            } else {
                Res::NotFound
            },
        _ => Res::NotFound,
    }
}

fn until_eol(mut data: &[u8]) -> Res<'_, Vec<u8>> {
    let mut result = vec![];

    while data.len() > 0 {
        match eol(data) {
            Res::NotFound => {
                result.push(data[0]);
                data = &data[1..];
            },
            Res::Found(r) => {
                return Res::found(result, r.remaining);
            },
            Res::Error => {
                return Res::Error;
            }
        }
    }

    // Reached the end of the buffer
    Res::found(result, data)
}

// 7.2.3
fn comment(mut data: &[u8]) -> Res<'_, Vec<u8>> {
    ascii!(data, ASCII_PERCENT_SIGN);

    let comment = block!(data, until_eol);
    Res::found(comment, data)
}

fn string_comment(mut data: &[u8]) -> Res<'_, String> {
    let comment = block!(data, comment);
    Res::string(comment, data)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Version {
    V1,
    V1_1,
    V1_2,
    V1_3,
    V1_4,
    V1_5,
    V1_6,
    V1_7,
    V1_8,
    Newer(String),
}

impl Version {
    fn newer(version: &str) -> Version {
        // TODO: maybe match on "PDF-1.X" and remove
        // the extra part?
        Version::Newer(version.to_string())
    }
}

// 7.5.2
fn version(mut data: &[u8]) -> Res<'_, Version> {
    let version_comment = block!(data, string_comment);
    if !version_comment.starts_with("PDF-1") {
        return Res::NotFound;
    }

    let result = match version_comment.as_ref() {
        "PDF-1.0" => Version::V1,
        "PDF-1.1" => Version::V1_1,
        "PDF-1.2" => Version::V1_2,
        "PDF-1.3" => Version::V1_3,
        "PDF-1.4" => Version::V1_4,
        "PDF-1.5" => Version::V1_5,
        "PDF-1.6" => Version::V1_6,
        "PDF-1.7" => Version::V1_7,
        "PDF-1.8" => Version::V1_8,
        v => Version::newer(v),
    };

    Res::found(result, data)
}

// 7.5.5
fn eof(mut data: &[u8]) -> Res<'_, ()> {
    let eof_comment = block!(data, string_comment);
    if eof_comment != "%EOF" {
        return Res::NotFound;
    }

    Res::found((), data)
}

fn exact<'a>(data: &'a [u8], expected: &'a str) -> Res<'a, ()> {
    if data.len() < expected.len() {
        return Res::NotFound;
    }

    let bytes = expected.as_bytes();
    for i in 0..expected.len() {
        if data[i] != bytes[i] {
            return Res::NotFound;
        }
    }

    Res::found((), &data[expected.len()..])
}

// 7.3.2
fn boolean(data: &[u8]) -> Res<'_, bool> {
    let is_true = exact(data, "true");
    if let Res::Found(result) = is_true {
        return Res::found(true, result.remaining);
    }

    let is_false = exact(data, "false");
    if let Res::Found(result) = is_false {
        return Res::found(false, result.remaining);
    }

    Res::NotFound
}

fn is_numeric_ascii(data: u8) -> bool {
    match data as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'
            | '+' | '-' => true,
        _ => false,
    }
}

fn is_float_ascii(data: u8) -> bool {
    is_numeric_ascii(data) || data == '.' as u8
}

// 7.3.3
fn integer(data: &[u8]) -> Res<'_, i64> {
    requires!(data, is_float_ascii);

    let mut i = 0;
    while data.len() > i && is_float_ascii(data[i]) {
        i += 1;
    }

    Res::i64(&data[0..i], &data[i..])
}

// 7.5.8.3
fn binary_integer(data: &[u8], size: usize) -> Res<'_, u64> {
    if size > 8 {
        return Res::Error;
    }

    if data.len() < size {
        return Res::NotFound;
    }

    let mut result: u64 = 0;
    for i in &data[..size] {
        result = (result << 8) + *i as u64;
    }

    Res::found(result, &data[size..])
}

fn nonnegative_integer(mut data: &[u8]) -> Res<'_, u64> {
    let result = block!(data, integer);
    if result < 0 {
        return Res::NotFound;
    }

    Res::found(result as u64, data)
}

// 7.3.3
fn float(data: &[u8]) -> Res<'_, f64> {
    requires!(data, is_float_ascii);

    let mut i = 0;
    while data.len() > i && is_float_ascii(data[i]) {
        i += 1;
    }

    Res::f64(&data[0..i], &data[i..])
}

fn is_octal_digit(data: u8) -> bool {
    match data as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' => true,
        _ => false,
    }
}

fn octal_char(data: &[u8]) -> Res<'_, u8> {
    let mut i = 0;

    while i < 3 && data.len() > i && is_octal_digit(data[i]) {
        i += 1;
    }

    if i == 0 {
        return Res::NotFound;
    }

    let octal = if i == 3 {
        (data[0], data[1], data[2])
    } else if i == 2 {
        (ASCII_ZERO, data[0], data[1])
    } else {
        (ASCII_ZERO, ASCII_ZERO, data[0])
    };

    // XXX
    let result = (octal.0 - ASCII_ZERO) * 64 + (octal.1 - ASCII_ZERO) * 8
            + (octal.2 - ASCII_ZERO);

    Res::found(result, &data[i..])
}

// 7.3.4.2
fn string_escape(data: &[u8]) -> Res<'_, u8> {
    if data.len() < 2 || data[0] != ASCII_REVERSE_SOLIDUS {
        return Res::NotFound;
    }

    let octal = octal_char(&data[1..]);
    if octal.is_found() {
        return octal;
    }

    let eol = eol(&data[1..]);
    if let Res::Found(r) = eol {
        return Res::found(ASCII_SPACE, r.remaining);
    }

    let result = match data[1] as char {
        'n' => ASCII_LINE_FEED,
        'r' => ASCII_CARRIAGE_RETURN,
        't' => ASCII_HORIZONTAL_TAB,
        'b' => ASCII_BACKSPACE,
        'f' => ASCII_FORM_FEED,
        '(' => ASCII_LEFT_PARENTHESIS,
        ')' => ASCII_RIGHT_PARENTHESIS,
        '\\' => ASCII_REVERSE_SOLIDUS,
        x => x as u8, // Reverse solidus is ignored otherwise
    };

    Res::found(result, &data[2..])
}

// 7.3.4.2
fn literal_string(mut data: &[u8]) -> Res<'_, Vec<u8>> {
    ascii!(data, ASCII_LEFT_PARENTHESIS);

    let mut result = vec![];
    let mut balance = 1;

    while data.len() > 0 {
        let escape = string_escape(data);
        if let Res::Found(r) = escape {
            result.push(r.data);
            data = r.remaining;
            continue;
        }

        let c = data[0];
        data = &data[1..];

        match c {
            ASCII_RIGHT_PARENTHESIS => {
                balance -= 1;
                if balance == 0 {
                    break;
                }
            },
            ASCII_LEFT_PARENTHESIS => {
                balance += 1;
            },
            _ => {}
        }

        result.push(c);
    }

    if balance != 0 {
        // Only balanced parentheses are allowed
        Res::Error
    } else {
        Res::found(result, data)
    }
}

fn is_whitespace_ascii(data: u8) -> bool {
    match data {
        ASCII_SPACE
            | ASCII_HORIZONTAL_TAB
            | ASCII_CARRIAGE_RETURN
            | ASCII_LINE_FEED
            | ASCII_FORM_FEED => true,
        _ => false,
    }
}

fn is_hex_ascii(data: u8) -> bool {
    match data {
        ASCII_ZERO
            | ASCII_ONE
            | ASCII_TWO
            | ASCII_THREE
            | ASCII_FOUR
            | ASCII_FIVE
            | ASCII_SIX
            | ASCII_SEVEN
            | ASCII_EIGHT
            | ASCII_NINE
            | ASCII_A
            | ASCII_B
            | ASCII_C
            | ASCII_D
            | ASCII_E
            | ASCII_F
            | ASCII_A_LOWERCASE
            | ASCII_B_LOWERCASE
            | ASCII_C_LOWERCASE
            | ASCII_D_LOWERCASE
            | ASCII_E_LOWERCASE
            | ASCII_F_LOWERCASE => true,
        _ => false,
    }
}

fn uppercase_hex(data: u8) -> u8 {
    match data {
        ASCII_A_LOWERCASE => ASCII_A,
        ASCII_B_LOWERCASE => ASCII_B,
        ASCII_C_LOWERCASE => ASCII_C,
        ASCII_D_LOWERCASE => ASCII_D,
        ASCII_E_LOWERCASE => ASCII_E,
        ASCII_F_LOWERCASE => ASCII_F,
        x => x,
    }
}

fn ascii_to_hex(data: u8) -> u8 {
    match data {
        ASCII_ZERO        => 0x0,
        ASCII_ONE         => 0x1,
        ASCII_TWO         => 0x2,
        ASCII_THREE       => 0x3,
        ASCII_FOUR        => 0x4,
        ASCII_FIVE        => 0x5,
        ASCII_SIX         => 0x6,
        ASCII_SEVEN       => 0x7,
        ASCII_EIGHT       => 0x8,
        ASCII_NINE        => 0x9,
        ASCII_A           => 0xA,
        ASCII_B           => 0xB,
        ASCII_C           => 0xC,
        ASCII_D           => 0xD,
        ASCII_E           => 0xE,
        ASCII_F           => 0xF,
        _ => unreachable!(),
    }
}

// 7.3.4.3
fn hex_string(mut data: &[u8]) -> Res<'_, Vec<u8>> {
    ascii!(data, ASCII_LESS_THAN_SIGN);

    let mut result = vec![];
    while data.len() == 0 || data[0] != ASCII_GREATER_THAN_SIGN {
        if is_whitespace_ascii(data[0]) {
            // Whitespace is ignored in hex strings
        } else if is_hex_ascii(data[0]) {
            result.push(uppercase_hex(data[0]));
        } else {
            return Res::Error;
        }
        data = &data[1..];
    }

    ascii!(data, ASCII_GREATER_THAN_SIGN);

    let mut bytes = vec![];
    let mut hex = &result[..];
    loop {
        if let Res::Found(r) = ascii_array_to_hex(hex) {
            bytes.push(r.data);
            hex = r.remaining;
        } else {
            break;
        }
    }

    Res::found(bytes, data)
}

fn ascii_array_to_hex(data: &[u8]) -> Res<'_, u8> {
    for i in 0..cmp::min(2, data.len()) {
        if !is_hex_ascii(data[i]) {
            return Res::NotFound;
        }
    }
    if data.len() >= 2 {
        Res::found(ascii_to_hex(uppercase_hex(data[0])) * 0x10
            + ascii_to_hex(uppercase_hex(data[1])), &data[2..])
    } else if data.len() == 1 {
        Res::found(ascii_to_hex(uppercase_hex(data[0])) * 0x10,
            &data[1..])
    } else {
        Res::NotFound
    }
}

// 7.3.4
fn string(data: &[u8]) -> Res<'_, Vec<u8>> {
    let r = hex_string(data);
    if r.is_found() {
        return r;
    }

    return literal_string(data);
}

fn identifier_escape(mut data: &[u8]) -> Res<'_, u8> {
    ascii!(data, ASCII_NUMBER_SIGN);

    if data.len() < 2 {
        // Ident escape need to be two hex characters
        return Res::Error;
    }

    return ascii_array_to_hex(data);
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Key {
    object: u64,
    generation: u64,
}

impl Key {
    fn new(object: u64, generation: u64) -> Key {
        Key {
            object,
            generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Definition {
    key: Key,
    object: PdfObject,
}

impl Definition {
    fn new(key: Key, object: PdfObject) -> Definition {
        Definition {
            key,
            object,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    data: Vec<u8>,
    metadata: StreamMetadata,
}

impl Stream {
    pub fn new(data: &[u8], metadata: StreamMetadata) -> Stream {
        Stream {
            data: data.to_vec(),
            metadata,
        }
    }

    fn apply_flate_decode(&mut self) -> Result<(), String> {
        let mut data = vec![];
        mem::swap(&mut self.data, &mut data);

        let mut decoded;
        {
            let mut reader = BitReader::new(Box::new(Cursor::new(
                data)));
            decoded = Cursor::new(vec![]);
            rfc1950(&mut reader, &mut decoded)
                .map_err(|e| e.to_string())?;
        }

        mem::swap(&mut self.data, &mut decoded.into_inner());
        Ok(())
    }

    pub fn apply_filters(&mut self) -> Result<(), String> {
        for filter in self.metadata.filters.clone() {
            match filter {
                Filter::FlateDecode => self.apply_flate_decode()?,
                _ => return Err(format!("Unimplemented filter {:?}.", filter)),
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    ASCIIHexDecode,
    ASCII85Decode,
    LZWDecode,
    FlateDecode,
    RunLengthDecode,
    CCITTFaxDecode,
    JBIG2Decode,
    DCTDecode,
    JPXDecode,
    Crypt,
}

impl Filter {
    fn from(obj: &PdfObject) -> Option<Filter> {
        Some(match obj.as_identifier()? {
            "ASCIIHexDecode" => Filter::ASCIIHexDecode,
            "ASCII85Decode" => Filter::ASCII85Decode,
            "LZWDecode" => Filter::LZWDecode,
            "FlateDecode" => Filter::FlateDecode,
            "RunLengthDecode" => Filter::RunLengthDecode,
            "CCITTFaxDecode" => Filter::CCITTFaxDecode,
            "JBIG2Decode" => Filter::JBIG2Decode,
            "DCTDecode" => Filter::DCTDecode,
            "JPXDecode" => Filter::JPXDecode,
            "Crypt" => Filter::Crypt,
            _ => return None,
        })
    }

    fn from_vec(obj: &PdfObject) -> Option<Vec<Filter>> {
        if obj.as_identifier().is_some() {
            Some(vec![Filter::from(obj)?])
        } else {
            Some(obj.as_array()?.iter()
                .filter_map(Filter::from)
                .collect())
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct StreamMetadata {
    length: usize,
    filters: Vec<Filter>,
    dictionary: PdfDictionary,
    // TODO: the rest of the fields
}

impl StreamMetadata {
    fn from(dictionary: PdfDictionary) -> Option<StreamMetadata> {
        match dictionary.integer("Length") {
            Some(length) => if length >= 0 {
                Some(StreamMetadata {
                    length: length as usize,
                    filters: dictionary.get("Filter")
                        .and_then(Filter::from_vec)
                        .unwrap_or(vec![]),
                    dictionary,
                })
            } else {
                None
            },
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum PdfObject {
    Array(Vec<PdfObject>),
    Boolean(bool),
    Reference(Key),
    Dictionary(PdfDictionary),
    Float(f64),
    Identifier(String),
    Integer(i64),
    Stream(Stream),
    Null,
    String(Vec<u8>),
}

pub trait OptionalFrom
where Self: Sized {
    fn from(obj: &PdfObject, pdf: &Pdf) -> Option<Self>;
}

#[macro_export]
macro_rules! pdf_struct {
    (optional, $name:ident, $type:ty, $data:expr, $pdf:expr, $key:expr) => {
        let $name: Option<$type> = $data.get($key)
            .and_then(|o| OptionalFrom::from(o, $pdf));
    };
    ($name:ident, $type:ty, $data:expr, $pdf:expr, $key:expr) => {
        let $name: $type = OptionalFrom::from($data.get($key)?, $pdf)?;
    };
    {
        $name:ident {
            required: [
                $($field_name:ident: $type:ty, $key:expr),*
            ],
            optional: [
                $($opt_field_name:ident: $opt_type:ty, $opt_key:expr),*
            ]
        }
    } => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $($field_name: $type,)*
            $($opt_field_name: Option<$opt_type>,)*
        }

        impl OptionalFrom for $name {
            fn from(obj: &PdfObject, pdf: &parser::Pdf) -> Option<$name> {
                let data = obj.as_dictionary(pdf)?;
                $(pdf_struct!($field_name, $type, data, pdf, $key);)*
                $(pdf_struct!(optional, $opt_field_name, $opt_type, data, pdf,
                              $opt_key);)*
                Some($name {
                    $($field_name,)*
                    $($opt_field_name,)*
                })
            }
        }
    }
}

impl OptionalFrom for u64 {
    fn from(obj: &PdfObject, _: &Pdf) -> Option<Self> {
        obj.as_unsigned()
    }
}

impl OptionalFrom for i64 {
    fn from(obj: &PdfObject, _: &Pdf) -> Option<Self> {
        obj.as_integer()
    }
}

impl OptionalFrom for f64 {
    fn from(obj: &PdfObject, _: &Pdf) -> Option<Self> {
        obj.as_float()
    }
}

impl OptionalFrom for Vec<u64> {
    fn from(obj: &PdfObject, _: &Pdf) -> Option<Self> {
        Some(obj.as_unsigned_array()?.collect())
    }
}

impl OptionalFrom for Vec<i64> {
    fn from(obj: &PdfObject, _: &Pdf) -> Option<Self> {
        Some(obj.as_integer_array()?.collect())
    }
}

impl OptionalFrom for Vec<u8> {
    fn from(obj: &PdfObject, _: &Pdf) -> Option<Self> {
        obj.as_string().map(|s| s.to_vec())
    }
}

impl OptionalFrom for String {
    fn from(obj: &PdfObject, _: &Pdf) -> Option<Self> {
        obj.as_string()
           .and_then(|b| str::from_utf8(b).ok())
           .or_else(|| obj.as_identifier())
           .map(str::to_string)
    }
}

impl PdfObject {
    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_unsigned_array(&self) -> Option<impl Iterator<Item = u64> + '_> {
        Some(self.as_array()?.iter().filter_map(PdfObject::as_unsigned))
    }

    pub fn as_integer_array(&self) -> Option<impl Iterator<Item = i64> + '_> {
        Some(self.as_array()?.iter().filter_map(PdfObject::as_integer))
    }

    pub fn as_float_array(&self) -> Option<impl Iterator<Item = f64> + '_> {
        Some(self.as_array()?.iter().filter_map(PdfObject::as_float))
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            PdfObject::Boolean(x) => Some(*x),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<&Key> {
        match self {
            PdfObject::Reference(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_dictionary<'a>(&'a self, pdf: &'a Pdf) -> Option<&'a PdfDictionary> {
        match self {
            PdfObject::Dictionary(x) => Some(x),
            PdfObject::Reference(r) => pdf.resolve(&r).as_dictionary(pdf),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            PdfObject::Float(x) => Some(*x),
            PdfObject::Integer(x) => Some(*x as f64),
            _ => None,
        }
    }

    pub fn as_identifier(&self) -> Option<&str> {
        match self {
            PdfObject::Identifier(x) => Some(x.as_str()),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(x) => Some(*x),
            _ => None,
        }
    }

    pub fn as_unsigned(&self) -> Option<u64> {
        let x = self.as_integer()?;
        if x <= 0 {
            None
        } else {
            Some(x as u64)
        }
    }

    pub fn as_string(&self) -> Option<&[u8]> {
        match self {
            PdfObject::String(x) => Some(x.as_slice()),
            _ => None,
        }
    }
}

// 7.3.5
fn identifier(mut data: &[u8]) -> Res<'_, String> {
    ascii!(data, ASCII_SOLIDUS);

    let mut result = vec![];
    while data.len() > 0 && data[0] >= ASCII_EXCLAMATION_MARK
            && data[0] <= ASCII_TILDE
            // XXX Not sure about these but otherwise it breaks arrays and maps
            && data[0] != ASCII_SOLIDUS
            && data[0] != ASCII_LEFT_PARENTHESIS
            && data[0] != ASCII_RIGHT_SQUARE_BRACKET
            && data[0] != ASCII_LEFT_SQUARE_BRACKET
            && data[0] != ASCII_LESS_THAN_SIGN
            && data[0] != ASCII_GREATER_THAN_SIGN {
        if let Res::Found(escape) = identifier_escape(data) {
            data = escape.remaining;
            result.push(escape.data);
            continue;
        }

        result.push(data[0]);
        data = &data[1..];
    }

    Res::string(result, data)
}

fn reference_header(mut data: &[u8]) -> Res<'_, Key> {
    let object = block!(data, integer);
    data = consume_whitespace(data);

    let generation = block!(data, integer);

    if object < 1 || generation < 0 {
        return Res::NotFound;
    }

    Res::found(Key::new(object as u64, generation as u64), data)
}

// 7.3.10
fn reference(mut data: &[u8]) -> Res<'_, Key> {
    let reference = block!(data, reference_header);

    data = consume_whitespace(data);

    ascii!(data, ASCII_R);

    Res::found(reference, data)
}

// 7.3.10
fn definition(mut data: &[u8]) -> Res<'_, Definition> {
    let reference = block!(data, reference_header);
    data = consume_whitespace(data);

    exact!(data, "obj");
    data = consume_whitespace(data);

    let obj = block!(data, object);
    data = consume_whitespace(data);

    exact!(data, "endobj");

    Res::found(Definition::new(reference, obj), data)
}

fn stream_definition<'a, F>(mut data: &'a [u8],
                            resolve: &mut F) -> Res<'a, Definition>
where F: FnMut(&Key) -> PdfObject {
    let reference = block!(data, reference_header);
    data = consume_whitespace(data);

    exact!(data, "obj");
    data = consume_whitespace(data);

    let stream = block!(data, stream, resolve);
    data = consume_whitespace(data);

    exact!(data, "endobj");

    Res::found(Definition::new(reference,
        PdfObject::Stream(stream)), data)
}

// 7.3.8.1
fn stream<'a, F>(mut data: &'a [u8], resolve: &mut F) -> Res<'a, Stream>
where F: FnMut(&Key) -> PdfObject {
    let dict = resolve_dictionary(block!(data, dictionary), resolve);

    let metadata;
    if let Some(d) = StreamMetadata::from(dict) {
        metadata = d;
    } else {
        return Res::NotFound;
    }

    data = consume_whitespace(data);

    exact!(data, "stream");
    block!(data, eol);

    if data.len() < metadata.length {
        return Res::Error;
    }

    let length = metadata.length;
    let result = Stream::new(&data[0..length], metadata);
    data = &data[length..];

    optional!(data, eol);
    exact!(data, "endstream");

    Res::found(result, data)
}

fn object(data: &[u8]) -> Res<'_, PdfObject> {
    if let Res::Found(r) = boolean(data) {
        return Res::found(PdfObject::Boolean(r.data), r.remaining);
    }
    if let Res::Found(r) = null(data) {
        return Res::found(PdfObject::Null, r.remaining);
    }
    if let Res::Found(r) = reference(data) {
        return Res::found(PdfObject::Reference(r.data), r.remaining);
    }
    if let Res::Found(r) = integer(data) {
        return Res::found(PdfObject::Integer(r.data), r.remaining);
    }
    if let Res::Found(r) = float(data) {
        return Res::found(PdfObject::Float(r.data), r.remaining);
    }
    if let Res::Found(r) = string(data) {
        return Res::found(PdfObject::String(r.data), r.remaining);
    }
    if let Res::Found(r) = identifier(data) {
        return Res::found(PdfObject::Identifier(r.data), r.remaining);
    }
    if let Res::Found(r) = array(data) {
        return Res::found(PdfObject::Array(r.data), r.remaining);
    }
    if let Res::Found(r) = dictionary(data) {
        return Res::found(PdfObject::Dictionary(r.data), r.remaining);
    }

    Res::NotFound
}

/// Consumes whitespace or comments, wether they are there or not
fn consume_whitespace(mut data: &[u8]) -> &[u8] {
    while data.len() > 0 {
        if is_whitespace(data[0]) {
            data = &data[1..];
            continue;
        }

        if let Res::Found(r) = comment(data) {
            data = r.remaining;
            continue;
        }

        break;
    }

    data
}

// 7.3.6
fn array(mut data: &[u8]) -> Res<'_, Vec<PdfObject>> {
    ascii!(data, ASCII_LEFT_SQUARE_BRACKET);
    data = consume_whitespace(data);

    let mut result = vec![];
    loop {
        if let Res::Found(o) = object(data) {
            result.push(o.data);
            data = o.remaining;
        } else {
            data = consume_whitespace(data);
            break;
        }
        data = consume_whitespace(data);
    }

    ascii!(data, ASCII_RIGHT_SQUARE_BRACKET);

    Res::found(result, data)
}

// 7.3.9
fn null(data: &[u8]) -> Res<'_, ()> {
    exact(data, "null")
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfDictionary {
    pub data: HashMap<String, PdfObject>,
}

impl PdfDictionary {
    pub fn new(data: HashMap<String, PdfObject>) -> PdfDictionary {
        PdfDictionary { data }
    }

    pub fn get(&self, key: &str) -> Option<&PdfObject> {
        self.data.get(key)
    }

    pub fn integer(&self, key: &str) -> Option<i64> {
        self.data.get(key).and_then(PdfObject::as_integer)
    }

    pub fn unsigned(&self, key: &str) -> Option<u64> {
        self.data.get(key).and_then(PdfObject::as_unsigned)
    }

    pub fn identifier(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(PdfObject::as_identifier)
    }

    pub fn float(&self, key: &str) -> Option<f64> {
        self.data.get(key).and_then(PdfObject::as_float)
    }

    pub fn boolean(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(PdfObject::as_boolean)
    }

    pub fn array(&self, key: &str) -> Option<&[PdfObject]> {
        self.data.get(key).and_then(PdfObject::as_array)
    }

    pub fn dictionary<'a>(&'a self, key: &str, pdf: &'a Pdf)
            -> Option<&'a PdfDictionary> {
        self.data.get(key).and_then(|obj| obj.as_dictionary(pdf))
    }

    pub fn reference_array(&self, key: &str)
            -> Option<impl Iterator<Item = &Key>> {
        Some(self.array(key)?.iter()
            .filter_map(PdfObject::as_reference))
    }

    pub fn identifier_array(&self, key: &str)
            -> Option<impl Iterator<Item = &str>> {
        Some(self.array(key)?.iter()
            .filter_map(PdfObject::as_identifier))
    }

    pub fn integer_array(&self, key: &str)
            -> Option<impl Iterator<Item = i64> + '_> {
        Some(self.array(key)?.iter()
            .filter_map(PdfObject::as_integer))
    }

    /// Iterates through an array of references, resolves them and maps them to
    /// an object of type T using `map`. Returns `None` if either the element
    /// is not found or any of the references is not found.
    pub fn map_reference_array<T, F>(&self, key: &str, pdf: &Pdf, map: F)
            -> Option<Vec<T>>
    where F: Fn(&PdfDictionary, &Pdf) -> Option<T>
    {
        let mut result = vec![];
        for k in self.reference_array(key)? {
            result.push(
                map(pdf.resolve(&k).as_dictionary(pdf)?, pdf)?);
        }

        return Some(result);
    }
}

// 7.3.7
fn dictionary(mut data: &[u8]) -> Res<'_, PdfDictionary> {
    ascii!(data, ASCII_LESS_THAN_SIGN);
    ascii!(data, ASCII_LESS_THAN_SIGN);
    data = consume_whitespace(data);

    let mut result = HashMap::new();
    while data.len() > 0 {
        let key = repeat!(data, identifier);
        data = consume_whitespace(data);

        let value = block!(data, object);
        data = consume_whitespace(data);

        result.insert(key, value);
    }

    ascii!(data, ASCII_GREATER_THAN_SIGN);
    ascii!(data, ASCII_GREATER_THAN_SIGN);

    Res::found(PdfDictionary::new(result), data)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XrefType {
    Free,
    InUse,
    Compressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct XrefEntry {
    offset: usize,
    generation_number: u64,
    type_: XrefType,
}

fn fixed_integer(data: &[u8], length: usize) -> Res<'_, u64> {
    if data.len() < length {
        return Res::NotFound;
    }

    String::from_utf8((&data[0..length]).to_vec()).ok()
        .and_then(|s| u64::from_str(&s).ok())
        .map(|x| Res::found(x, &data[length..]))
        .unwrap_or(Res::NotFound)
}

// 7.5.4
fn xref_entry(mut data: &[u8]) -> Res<'_, XrefEntry> {
    if data.len() < 20 {
        return Res::NotFound;
    }

    let offset = block!(data, fixed_integer, 10) as usize;

    ascii!(data, ASCII_SPACE);

    let generation_number = block!(data, fixed_integer, 5);

    ascii!(data, ASCII_SPACE);

    let type_ = if data[0] == ASCII_N_LOWERCASE {
        XrefType::InUse
    } else if data[0] == ASCII_F_LOWERCASE {
        XrefType::Free
    } else {
        return Res::NotFound;
    };

    data = &data[1..];
    // TODO: this should only consume exactly 2 bytes
    data = consume_whitespace(data);

    return Res::found(XrefEntry { offset, generation_number, type_ }, data);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Xref {
    offset: usize,
    type_: XrefType,
    key: Key,
}

impl Xref {
    fn from(entry: XrefEntry, object_number: u64) -> Xref {
        Xref {
            offset: entry.offset,
            type_: entry.type_,
            key: Key {
                object: object_number,
                generation: entry.generation_number,
            },
        }
    }
}

// 7.5.4
fn xref_table(mut data: &[u8]) -> Res<'_, HashMap<u64, Xref>> {
    exact!(data, "xref");
    data = consume_whitespace(data);

    let mut xref_table = HashMap::new();
    loop {
        let object_number = repeat!(data, nonnegative_integer);
        data = consume_whitespace(data);

        let entries = block!(data, nonnegative_integer);
        data = consume_whitespace(data);

        for i in 0..entries {
            let xref_entry = block!(data, xref_entry);
            let xref = Xref::from(xref_entry, object_number as u64 + i);
            xref_table.insert(xref.key.object, xref);
        }
    }

    Res::found(xref_table, data)
}

// 7.5.8.2
fn xref_binary_entry<'a>(mut data: &'a [u8], w: &[usize]) -> Res<'a, XrefEntry> {
    let type_ = block!(data, binary_integer, w[0]);
    let offset = block!(data, binary_integer, w[1]) as usize;
    let generation_number = block!(data, binary_integer, w[2]);

    let xref_type = match type_ {
        0 => XrefType::Free,
        1 => XrefType::InUse,
        2 => XrefType::Compressed,
        // XXX: this should be a ref to null
        _ => XrefType::Free,
    };

    Res::found(XrefEntry { offset, generation_number, type_: xref_type}, data)
}

// 7.5.8.2
fn xref_binary_table<'a>(mut data: &'a [u8], w: &[usize]) -> Res<'a, HashMap<u64, Xref>> {
    let mut xref_table = HashMap::new();
    let mut object_number = 0;

    while data.len() > 0 {
        let xref_entry = block!(data, xref_binary_entry, w);
        let xref = Xref::from(xref_entry, object_number);
        xref_table.insert(xref.key.object, xref);
        object_number += 1;
    }

    Res::found(xref_table, data)
}

// 7.5.8.1
fn xref_stream(mut data: &[u8]) -> Res<'_, HashMap<u64, Xref>> {
    let definition = block!(data, stream_definition, &mut |r| PdfObject::Reference(*r));

    match definition.object {
        PdfObject::Stream(mut stream) => {
            let w: Vec<usize> = stream.metadata.dictionary.integer_array("W")
                .map(|it| it.map(|x| x as usize).collect())
                .unwrap_or_else(|| vec![]);

            stream.apply_filters()?;

            let mut _stream_data = &stream.data[..];
            let xref = block!(_stream_data, xref_binary_table, w.as_slice());
            Res::found(xref, data)
        },
        _ => { Res::Error },
    }
}

// 7.5.5
fn startxref(mut data: &[u8]) -> Res<'_, u64> {
    exact!(data, "startxref");
    data = consume_whitespace(data);

    let result = block!(data, nonnegative_integer);
    Res::found(result, data)
}

// 7.5.5
fn trailer(mut data: &[u8]) -> Res<'_, PdfDictionary> {
    exact!(data, "trailer");
    data = consume_whitespace(data);

    let result = block!(data, dictionary);
    Res::found(result, data)
}

#[derive(Debug)]
pub struct Pdf {
    version: Version,
    objects: HashMap<u64, PdfObject>,
}

impl Pdf {
    pub fn resolve(&self, key: &Key) -> &PdfObject {
        self.objects.get(&key.object).unwrap_or(&PdfObject::Null)
    }

    pub fn objects(&self) -> &HashMap<u64, PdfObject> {
        &self.objects
    }
}

// 7.5
#[allow(unused_assignments)]
fn pdf(mut data: &[u8]) -> Res<'_, Pdf> {
    let original_data = data;

    if data.len() < 1 {
        return Res::NotFound;
    }

    // First, let's find the `startxref` reference at the end of the file.
    let mut end = data.len() - 1;
    let startxref_obj = loop {
        if data.len() - end > 100 {
            // The offset can only be so many bytes, if we got this far this is not a
            // valid PDF file.
            return Res::NotFound;
        }

        if let Res::Found(xref) = startxref(&data[end..]) {
            break xref;
        } else {
            end -= 1;
            continue;
        }
    };

    let mut remaining = startxref_obj.remaining;

    // Let's make sure that the end of file is valid
    block!(remaining, eol);
    block!(remaining, eof);

    // We should be at the end of the file now
    if remaining != &[] {
        return Res::NotFound;
    }

    let startxref_index = startxref_obj.data as usize;

    if data.len() < startxref_index {
        // startxref is not a valid index
        return Res::Error;
    }

    let mut xref_data = &data[startxref_index..];

    let has_binary_xref;
    let xref;
    // The xref table can either be explicit on in a stream object
    if let Res::Found(r) = xref_table(xref_data) {
        xref = r.data;
        xref_data = r.remaining;
        block!(xref_data, trailer);
        xref_data = consume_whitespace(xref_data);

        // We should be back at startxref now
        block!(xref_data, startxref);
        has_binary_xref = false;
    } else {
        xref = block!(xref_data, xref_stream);
        has_binary_xref = true;
    }

    let version = block!(data, version);
    data = consume_whitespace(data);

    let mut objects = HashMap::new();

    loop {
        let mut result;
        if let Res::Found(r) = stream_definition(data, &mut |k| {
            resolve(k, &xref, &mut objects, original_data).clone()
        }) {
            data = r.remaining;
            result = r.data;

            if let PdfObject::Stream(ref mut s) = result.object {
                // TODO: actually check if applying filters works
                let _ = s.apply_filters();
                if s.metadata.filters.contains(&Filter::FlateDecode) {
                    let string = String::from_utf8(s.clone().data);
                    if let Ok(x) = string {
                        println!("{}", x);
                    }
                }
            }
        } else {
            result = repeat!(data, definition);
        }

        objects.insert(result.key.object, result.object);
        data = consume_whitespace(data);
    }

    // After all the definitions we should be back at the xref table or at startxref
    if has_binary_xref {
        exact!(data, "startxref");
    } else {
        exact!(data, "xref");
    }

    Res::found(Pdf {
        version,
        objects,
    }, &[])
}

fn resolve<'a>(key: &Key, xref: &HashMap<u64, Xref>,
           objects: &'a mut HashMap<u64, PdfObject>,
           data: &[u8]) -> &'a PdfObject {
    if objects.contains_key(&key.object) {
        return objects.get(&key.object).unwrap();
    }

    let offset = xref[&key.object].offset;
    let resolved_data = &data[offset..];

    match definition(resolved_data) {
        Res::Found(x) => {
            if x.data.key != *key {
                panic!("Expected {:?} but found {:?}", key, x.data.key);
            }
            objects.insert(x.data.key.object, x.data.object);
            objects.get(&key.object).unwrap()
        }
        Res::NotFound | Res::Error => &PdfObject::Null,
    }
}

pub fn parse_pdf(data: &[u8]) -> Result<Pdf, String> {
    let result = pdf(data);

    match result {
        Res::Found(r) => Ok(r.data),
        Res::NotFound | Res::Error =>
                Err("Could not parse file.".to_string()),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test {
        ($name: ident, $subject: ident) => {
            fn $name(data: &str, expected: &str, remaining: &str) {
                let result = $subject(data.as_bytes()).unwrap();
                assert_eq!(str::from_utf8(&result.data).unwrap(), expected);
                assert_eq!(from_bytes(result.remaining).as_str(), remaining);
            }
        };
        ($name: ident, $subject: ident, String) => {
            fn $name(data: &str, expected: &str, remaining: &str) {
                let result = $subject(data.as_bytes()).unwrap();
                assert_eq!(result.data.to_string(), expected);
                assert_eq!(from_bytes(result.remaining).as_str(), remaining);
            }
        };
        ($name: ident, $subject: ident, $expected: ty) => {
            fn $name(data: &str, expected: $expected, remaining: &str) {
                let result = $subject(data.as_bytes()).unwrap();
                assert_eq!(result.data, expected);
                assert_eq!(from_bytes(result.remaining).as_str(), remaining);
            }
        };
        ($name: ident, $subject: ident, $expected: ident, $map: expr) => {
            fn $name(data: &str, expected: $expected, remaining: &str) {
                let result = $subject(data.as_bytes()).unwrap();
                assert_eq!($map(&result), expected);
                assert_eq!(from_bytes(result.remaining).as_str(), remaining);
            }
        }
    }

    impl XrefEntry {
        fn new(offset: usize, generation_number: u64,
                   type_: XrefType) -> XrefEntry {
            XrefEntry {
                offset,
                generation_number,
                type_,
            }
        }
    }

    impl PdfObject {
        fn identifier(data: &str) -> PdfObject {
            PdfObject::Identifier(data.to_string())
        }

        fn string(data: &str) -> PdfObject {
            PdfObject::String(data.as_bytes().to_vec())
        }

        fn reference(object: u64, generation: u64) -> PdfObject {
            PdfObject::Reference(Key::new(object, generation))
        }
    }

    impl <'a, T> Res<'a, T> {
        fn unwrap(self) -> Found<'a, T> {
            if let Res::Found(result) = self {
                return result;
            }

            panic!("This is not a Res::Found");
        }
    }

    impl Xref {
        fn new(offset: usize, object_number: u64, generation_number: u64,
                type_: XrefType) -> Xref {
            Xref {
                offset,
                type_,
                key: Key {
                    object: object_number,
                    generation: generation_number,
                },
            }
        }
    }

    fn from_bytes(data: &[u8]) -> String {
        String::from_utf8(data.to_vec()).unwrap()
    }

    #[test]
    fn test_is_whitespace() {
        assert!(is_whitespace(' ' as u8));
        assert!(is_whitespace('\t' as u8));
        assert!(is_whitespace('\r' as u8));
        assert!(is_whitespace('\n' as u8));
        assert!(is_whitespace('\0' as u8));
        assert!(!is_whitespace('a' as u8));
        assert!(!is_whitespace('b' as u8));
    }

    fn until_eol_test(data: &str, expected: &str, remaining: &str) {
        let result = until_eol(data.as_bytes()).unwrap();

        assert_eq!(from_bytes(&result.data[..]).as_str(), expected);
        assert_eq!(from_bytes(result.remaining).as_str(), remaining);
    }

    #[test]
    fn test_until_eol() {
        until_eol_test("\r", "\r", "");
        until_eol_test("\n", "", "");
        until_eol_test("\ntest", "", "test");
        until_eol_test("test\nasd", "test", "asd");
        until_eol_test("test\r\nasd", "test", "asd");
        until_eol_test("test", "test", "");
    }

    test!(string_comment_test, string_comment, String);

    #[test]
    fn test_comment() {
        string_comment_test("%this is a comment", "this is a comment", "");
        string_comment_test("%this is% a comment", "this is% a comment", "");
        string_comment_test("%comment\nnot comment", "comment", "not comment");
        string_comment_test("%\nnot comment", "", "not comment");

        assert_eq!(Res::NotFound, comment("not a comment".as_bytes()));
        assert_eq!(Res::NotFound, comment("a%nope".as_bytes()));
        assert_eq!(Res::NotFound, comment("".as_bytes()));
    }

    test!(boolean_test, boolean, bool);

    #[test]
    fn test_boolean_object() {
        assert_eq!(boolean("either".as_bytes()), Res::NotFound);
        assert_eq!(boolean("".as_bytes()), Res::NotFound);
        assert_eq!(boolean("%true".as_bytes()), Res::NotFound);

        boolean_test("true", true, "");
        boolean_test("false", false, "");
        boolean_test("trueasd", true, "asd");
        boolean_test("true%test", true, "%test");
    }

    test!(integer_test, integer, i64);

    #[test]
    fn test_integer_object() {
        assert_eq!(integer("123.123".as_bytes()), Res::NotFound);

        integer_test("123abc", 123, "abc");
        integer_test("123", 123, "");
        integer_test("+123", 123, "");
        integer_test("-123", -123, "");
        integer_test("0", 0, "");
    }

    test!(float_test, float, f64);

    #[test]
    fn test_float() {
        float_test("123abc", 123.0, "abc");
        float_test("123", 123.0, "");
        float_test("+123", 123.0, "");
        float_test("-123", -123.0, "");
        float_test("123.123", 123.123, "");
        float_test("123.0", 123.0, "");
        float_test(".123", 0.123, "");
        float_test("0", 0.0, "");
    }

    test!(octal_char_test, octal_char, u8);

    #[test]
    fn test_octal_char() {
        octal_char_test("000", 0, "");
        octal_char_test("010", ASCII_BACKSPACE, "");
        octal_char_test("040", 0x20, "");
        octal_char_test("175", '}' as u8, "");
        octal_char_test("175xxx", '}' as u8, "xxx");
    }

    test!(string_escape_test, string_escape, char,
        |r: &Found<u8>| r.data as char);

    #[test]
    fn test_string_escape() {
        // Standard escapes
        string_escape_test("\\n", '\n', "");
        string_escape_test("\\r", '\r', "");
        string_escape_test("\\nabc", '\n', "abc");

        // Octal escapes
        string_escape_test("\\000", '\0', "");
        string_escape_test("\\00", '\0', "");
        string_escape_test("\\0", '\0', "");
        string_escape_test("\\0R", '\0', "R");
        string_escape_test("\\53", '+', "");
        string_escape_test("\\53X", '+', "X");
        string_escape_test("\\053", '+', "");
        string_escape_test("\\175", '}', "");

        // Other characters remain as is
        string_escape_test("\\H", 'H', "");
        string_escape_test("\\A", 'A', "");

        // eol escapes to space
        string_escape_test("\\\n", ' ', "");
        string_escape_test("\\\r\n", ' ', "");
        string_escape_test("\\ ", ' ', "");
    }

    test!(literal_string_test, literal_string);

    #[test]
    fn test_literal_string() {
        literal_string_test("()", "", "");
        literal_string_test("(test)", "test", "");
        literal_string_test("(test\\n)", "test\n", "");
        literal_string_test("(test\\\nasd)", "test asd", "");
        literal_string_test("(test)rest", "test", "rest");
        literal_string_test("(test\\tred q)", "test\tred q", "");
        literal_string_test("(test\\\r\nfoo\\\nbar)", "test foo bar", "");
        literal_string_test("(🎉)", "🎉", "");
        literal_string_test("(\\🎉)", "🎉", "");
    }

    test!(hex_string_test, hex_string);

    #[test]
    fn test_hex_string() {
        hex_string_test("<>", "", "");
        hex_string_test("<0>", "\0", "");
        hex_string_test("<0>test", "\0", "test");
        hex_string_test("<54 45 53 54>", "TEST", "");
        hex_string_test("<54 45\t53\n54>", "TEST", "");
        hex_string_test("<707>", "pp", "");
        hex_string_test("<F09F8E89>", "🎉", "");
        hex_string_test("<4E6F762073686D6F7A206B6120706F702E>",
                        "Nov shmoz ka pop.", "");
    }

    test!(string_test, string);

    #[test]
    fn test_string() {
        string_test("<F09F8E89>", "🎉", "");
        string_test("(🎉)", "🎉", "");
        string_test("(Strings may contain balanced parentheses ( ) and
special characters (*!&}^% and so on).)", "Strings may contain balanced parentheses ( ) and\nspecial characters (*!&}^% and so on).", "");
    }

    test!(identifier_escape_test, identifier_escape, char,
            |r: &Found<u8>| r.data as char);

    #[test]
    fn test_identifier_escape() {
        identifier_escape_test("#28", '(', "");
        identifier_escape_test("#29", ')', "");
        identifier_escape_test("#20", ' ', "");
    }

    test!(identifier_test, identifier, String);

    #[test]
    fn test_identifier() {
        // 7.3.5, Table 4
        identifier_test("/Name1", "Name1", "");
        identifier_test("/ASomewhatLongerName", "ASomewhatLongerName", "");
        identifier_test("/A;Name_With-Various***Characters?", "A;Name_With-Various***Characters?", "");
        identifier_test("/1.2", "1.2", "");
        identifier_test("/$$", "$$", "");
        identifier_test("/@pattern", "@pattern", "");
        identifier_test("/.notdef", ".notdef", "");
        identifier_test("/lime#20Green", "lime Green", "");
        identifier_test("/paired#28#29parentheses", "paired()parentheses", "");
        identifier_test("/The_Key_of_F#23_Minor", "The_Key_of_F#_Minor", "");
        identifier_test("/A#42", "AB", "");
    }

    test!(object_test, object, PdfObject);

    #[test]
    fn test_object() {
        object_test("true", PdfObject::Boolean(true), "");
        object_test("false", PdfObject::Boolean(false), "");
        object_test("false true", PdfObject::Boolean(false), " true");
        object_test("549", PdfObject::Integer(549), "");
        object_test("549 1", PdfObject::Integer(549), " 1");
        object_test("3.14", PdfObject::Float(3.14), "");
        object_test("3.14 2", PdfObject::Float(3.14), " 2");
        object_test("(Ralph)", PdfObject::string("Ralph"), "");
        object_test("(Ralph) (test)", PdfObject::string("Ralph"), " (test)");
        object_test("1 0 R", PdfObject::reference(1, 0), "");
        object_test("/SomeName", PdfObject::identifier("SomeName"), "");
        object_test("/SomeName (Ralph)", PdfObject::identifier("SomeName"),
                    " (Ralph)");
        object_test("null", PdfObject::Null, "");
        object_test("[null]",
            PdfObject::Array(vec![PdfObject::Null]), "");
        object_test("[549 3.14 false (Ralph) /SomeName]",
            PdfObject::Array(vec![
                PdfObject::Integer(549),
                PdfObject::Float(3.14),
                PdfObject::Boolean(false),
                PdfObject::string("Ralph"),
                PdfObject::identifier("SomeName")]), "");
        object_test("<</A /B/C[]>>",
            PdfObject::Dictionary(PdfDictionary::new(
                [
                    ("A".to_string(), PdfObject::identifier("B")),
                    ("C".to_string(), PdfObject::Array(vec![])),
                ].iter().cloned().collect())), "");
    }

    test!(array_test, array, Vec<PdfObject>);

    #[test]
    fn test_array() {
        array_test("[[]]", vec![PdfObject::Array(vec![])], "");
        array_test("[[1]]", vec![PdfObject::Array(vec![
                PdfObject::Integer(1)])], "");
        array_test("[/test] a", vec![PdfObject::identifier("test")], " a");
        array_test("[\n\
                /test] a", vec![PdfObject::identifier("test")], " a");
        array_test("[549 3.14 false (Ralph) /SomeName]",
            vec![
                PdfObject::Integer(549),
                PdfObject::Float(3.14),
                PdfObject::Boolean(false),
                PdfObject::string("Ralph"),
                PdfObject::identifier("SomeName")], "");
    }

    fn dictionary_test(data: &str, expected: &[(String, PdfObject)],
                       remaining: &str) {
        let result = dictionary(data.as_bytes()).unwrap();
        let expected_map =
            PdfDictionary::new(expected.iter().cloned().collect());
        assert_eq!(result.data, expected_map);
        assert_eq!(from_bytes(result.remaining).as_str(), remaining);
    }

    #[test]
    fn test_dictionary() {
        dictionary_test("<<>>", &[], "");
        dictionary_test("<<\n\
            >>", &[], "");
        dictionary_test("<</A /B>>",
            &[("A".to_string(), PdfObject::identifier("B"))], "");
        dictionary_test("<< /Type /Example
            /Subtype /DictionaryExample
            /Version 0.01
            /IntegerItem 12
            /StringItem (a string)
            /ReferenceItem 12 0 R
            /Subdictionary <<
                    /Item1 0.4
                    /Item2 true
                    /LastItem (not!)
                    /VeryLastItem (OK)
                >>
            >>", &[
            ("Type".to_string(), PdfObject::identifier("Example")),
            ("Subtype".to_string(), PdfObject::identifier("DictionaryExample")),
            ("Version".to_string(), PdfObject::Float(0.01)),
            ("IntegerItem".to_string(), PdfObject::Integer(12)),
            ("StringItem".to_string(), PdfObject::string("a string")),
            ("ReferenceItem".to_string(), PdfObject::reference(12, 0)),
            ("Subdictionary".to_string(), PdfObject::Dictionary(PdfDictionary::new(
                [("Item1".to_string(), PdfObject::Float(0.4)),
                 ("Item2".to_string(), PdfObject::Boolean(true)),
                 ("LastItem".to_string(), PdfObject::string("not!")),
                 ("VeryLastItem".to_string(), PdfObject::string("OK"))
                ].iter().cloned().collect())))
        ], "");
    }

    test!(reference_test, reference, Key);

    #[test]
    fn test_reference() {
        assert_eq!(reference("0 0 R".as_bytes()), Res::NotFound);
        assert_eq!(reference("-1 0 R".as_bytes()), Res::NotFound);
        assert_eq!(reference("1 0 M".as_bytes()), Res::NotFound);

        reference_test("17 0 R", Key::new(17, 0), "");
        reference_test("1 0 R", Key::new(1, 0), "");
        reference_test("1 10 R", Key::new(1, 10), "");
        reference_test("1 10 Rtest", Key::new(1, 10), "test");
    }

    test!(definition_test, definition, Definition);

    #[test]
    fn test_definition() {
        definition_test("12 0 obj
                (Brilling)
            endobj",
            Definition::new(
                Key::new(12, 0),
                PdfObject::string("Brilling")), "");
    }

    fn stream_test(data: &str, expected: &str, remaining: &str, objects: HashMap<u64, PdfObject>) {
        let result = stream(data.as_bytes(), &mut |key|
            objects.get(&key.object).unwrap_or(&PdfObject::Null).clone()).unwrap();
        assert_eq!(from_bytes(&result.data.data[..]).as_str(), expected);
        assert_eq!(from_bytes(result.remaining).as_str(), remaining);
    }

    #[test]
    fn test_stream() {
        stream_test("<< /Length 12 >> \
            stream\n\
                123456789012\n\
            endstream", "123456789012", "", HashMap::new());
        stream_test("<< /Length 12 >> \
            stream\n\
                123456789012endstream", "123456789012", "", HashMap::new());
        stream_test("<< /Length 8 0 R >> % Reference length\n\
            stream\n\
                123456789012\n\
            endstream", "123456789012", "",
            [(8, PdfObject::Integer(12))]
                .iter().cloned().collect());
    }

    test!(version_test, version, Version);

    #[test]
    fn test_version() {
        version_test("%PDF-1.0", Version::V1, "");
        version_test("%PDF-1.1", Version::V1_1, "");
        version_test("%PDF-1.2", Version::V1_2, "");
        version_test("%PDF-1.3", Version::V1_3, "");
        version_test("%PDF-1.4", Version::V1_4, "");
        version_test("%PDF-1.5", Version::V1_5, "");
        version_test("%PDF-1.6", Version::V1_6, "");
        version_test("%PDF-1.7", Version::V1_7, "");
        version_test("%PDF-1.8", Version::V1_8, "");
        version_test("%PDF-1.9", Version::newer("PDF-1.9"), "");
    }

    fn fixed_integer_5(data: &[u8]) -> Res<'_, u64> {
        fixed_integer(data, 5)
    }

    test!(fixed_integer_5_test, fixed_integer_5, u64);

    fn fixed_integer_10(data: &[u8]) -> Res<'_, u64> {
        fixed_integer(data, 10)
    }

    test!(fixed_integer_10_test, fixed_integer_10, u64);

    #[test]
    fn test_fixed_integer() {
        fixed_integer_5_test("00000", 0, "");
        fixed_integer_5_test("00000 ", 0, " ");
        fixed_integer_5_test("12345", 12345, "");
        fixed_integer_10_test("0000000000", 0, "");
        fixed_integer_10_test("1234567890", 1234567890, "");
    }

    test!(xref_entry_test, xref_entry, XrefEntry);

    #[test]
    fn test_xref() {
        xref_entry_test("1234567890 12345 f\r\n",
                XrefEntry::new(1234567890, 12345, XrefType::Free), "");
        xref_entry_test("0000000003 00000 n\r\n",
                XrefEntry::new(3, 0, XrefType::InUse), "");
    }

    test!(xref_table_test, xref_table, HashMap<u64, Xref>);

    #[test]
    fn test_xref_table() {
        xref_table_test("xref", HashMap::new(), "");
        xref_table_test("xref\n\
            0 6\n\
            0000000003 65535 f\r\n\
            0000000017 00000 n\r\n\
            0000000081 00000 n\r\n\
            0000000000 00007 f\r\n\
            0000000331 00000 n\r\n\
            0000000409 00000 n\r\n",
        [(0, Xref::new(3,   0, 65535, XrefType::Free)),
         (1, Xref::new(17,  1, 0,     XrefType::InUse)),
         (2, Xref::new(81,  2, 0,     XrefType::InUse)),
         (3, Xref::new(0,   3, 7,     XrefType::Free)),
         (4, Xref::new(331, 4, 0,     XrefType::InUse)),
         (5, Xref::new(409, 5, 0,     XrefType::InUse)),
        ].iter().cloned().collect(), "");
        xref_table_test("xref\n\
            0 1\n\
            0000000000 65535 f\r\n\
            3 1\n\
            0000025325 00000 n\r\n\
            23 2\n\
            0000025518 00002 n\r\n\
            0000025635 00000 n\r\n\
            30 1\n\
            0000025777 00000 n\r\n",
        [(0,  Xref::new(0,     0,  65535, XrefType::Free)),
         (3,  Xref::new(25325, 3,  0,     XrefType::InUse)),
         (23, Xref::new(25518, 23, 2,     XrefType::InUse)),
         (24, Xref::new(25635, 24, 0,     XrefType::InUse)),
         (30, Xref::new(25777, 30, 0,     XrefType::InUse)),
        ].iter().cloned().collect(), "");
    }

    test!(eof_test, eof, ());

    #[test]
    fn test_eof() {
        eof_test("%%EOF", (), "");
        eof_test("%%EOF\n", (), "");
    }

    test!(startxref_test, startxref, u64);

    #[test]
    fn test_startxref() {
        startxref_test("startxref\n\
                18799", 18799, "");
        startxref_test("startxref\n\
                0", 0, "");
    }

    test!(trailer_test, trailer, PdfDictionary);

    #[test]
    fn test_trailer() {
        trailer_test("\
                trailer\n\
                << /Size 95 /Root 1 0 R /Info 2 0 R\n\
                >>",
                    PdfDictionary::new(
                    [("Root".to_string(), PdfObject::reference(1, 0)),
                     ("Size".to_string(), PdfObject::Integer(95)),
                     ("Info".to_string(), PdfObject::reference(2, 0))]
                        .iter().cloned().collect())
                , "");
    }

    #[test]
    fn test_binary_integer() {
        assert_eq!(binary_integer(&[0, 0xFF], 2).unwrap().data, 0xFF);
        assert_eq!(binary_integer(&[0xFF, 0x00], 2).unwrap().data, 0xFF00);
    }
}
