use std::cmp;
use std::str::FromStr;
use std::collections::HashMap;

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

type PdfDictionary = HashMap<String, PdfObject>;

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
    ($data: ident, $f: ident, $param: expr) => {
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
pub fn is_whitespace(data: u8) -> bool {
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
pub enum Res<'a, T> {
    Found(Found<'a, T>),
    NotFound,
    Error,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Found<'a, T> {
    data: T,
    remaining: &'a [u8],
}

impl <'a, T> Res<'a, T> {
    pub fn found(data: T, remaining: &[u8]) -> Res<T> {
        Res::Found(Found {
            data: data,
            remaining: remaining,
        })
    }

    pub fn is_found(&self) -> bool {
        match self {
            Res::Found(_) => true,
            _ => false,
        }
    }

    pub fn unwrap(self) -> Found<'a, T> {
        if let Res::Found(result) = self {
            return result;
        }

        panic!("This is not a Res::Found");
    }

    pub fn map<U, F> (self, mapper: F) -> Res<'a, U>
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
    pub fn i64(data: &[u8], remaining: &'a [u8]) -> Res<'a, i64> {
        Res::string(data.to_vec(), remaining)
            .map(|s| i64::from_str(&s).ok())
    }
}

impl <'a> Res<'a, f64> {
    pub fn f64(data: &[u8], remaining: &'a [u8]) -> Res<'a, f64> {
        Res::string(data.to_vec(), remaining)
            .map(|s| f64::from_str(&s).ok())
    }
}

impl <'a> Res<'a, String> {
    pub fn string(data: Vec<u8>, remaining: &'a [u8]) -> Res<'a, String> {
        String::from_utf8(data)
            .map(|s| Res::found(s, remaining))
            .unwrap_or(Res::found("ERROR: Unparsable string.".to_string(), remaining))
    }
}

fn eol<'a>(data: &'a [u8]) -> Res<'a, ()> {
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
            }
        ,
        _ => Res::NotFound,
    }
}

pub fn until_eol<'a>(mut data: &'a [u8]) -> Res<'a, Vec<u8>> {
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
pub fn comment<'a>(mut data: &'a [u8]) -> Res<'a, Vec<u8>> {
    ascii!(data, ASCII_PERCENT_SIGN);

    let comment = block!(data, until_eol);
    Res::found(comment, data)
}

pub fn string_comment<'a>(mut data: &'a [u8]) -> Res<'a, String> {
    let comment = block!(data, comment);
    Res::string(comment, data)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Version {
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
    pub fn newer(version: &str) -> Version {
        // TODO: maybe match on "PDF-1.X" and remove
        // the extra part?
        Version::Newer(version.to_string())
    }
}

// 7.5.2
pub fn version<'a>(mut data: &'a [u8]) -> Res<'a, Version> {
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
pub fn eof<'a>(mut data: &'a [u8]) -> Res<'a, ()> {
    let eof_comment = block!(data, string_comment);
    if eof_comment != "%EOF" {
        return Res::NotFound;
    }

    Res::found((), data)
}

fn exact<'a>(data: &'a [u8], expected: &str) -> Res<'a, ()> {
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
pub fn boolean<'a>(data: &'a [u8]) -> Res<'a, bool> {
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
pub fn integer<'a>(data: &'a [u8]) -> Res<'a, i64> {
    requires!(data, is_float_ascii);

    let mut i = 0;
    while data.len() > i && is_float_ascii(data[i]) {
        i += 1;
    }

    Res::i64(&data[0..i], &data[i..])
}

pub fn nonnegative_integer<'a>(mut data: &'a [u8]) -> Res<'a, u64> {
    let result = block!(data, integer);
    if result < 0 {
        return Res::NotFound;
    }

    Res::found(result as u64, data)
}

// 7.3.3
pub fn float<'a>(data: &'a [u8]) -> Res<'a, f64> {
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

fn octal_char<'a>(data: &'a [u8]) -> Res<'a, u8> {
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
fn string_escape<'a>(data: &'a [u8]) -> Res<'a, u8> {
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
fn literal_string<'a>(mut data: &'a [u8]) -> Res<'a, String> {
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
        Res::string(result, data)
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
fn hex_string<'a>(mut data: &'a [u8]) -> Res<'a, String> {
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

    Res::string(bytes, data)
}

fn ascii_array_to_hex<'a>(data: &'a [u8]) -> Res<'a, u8> {
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
pub fn string<'a>(data: &'a [u8]) -> Res<'a, String> {
    let r = hex_string(data);
    if r.is_found() {
        return r;
    }

    return literal_string(data);
}

fn identifier_escape<'a>(mut data: &'a [u8]) -> Res<'a, u8> {
    ascii!(data, ASCII_NUMBER_SIGN);

    if data.len() < 2 {
        // Ident escape need to be two hex characters
        return Res::Error;
    }

    return ascii_array_to_hex(data);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reference {
    object: u64,
    generation: u64,
}

impl Reference {
    pub fn new(object: u64, generation: u64) -> Reference {
        Reference {
            object: object,
            generation: generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Definition {
    reference: Reference,
    object: PdfObject,
}

impl Definition {
    pub fn new(reference: Reference, object: PdfObject) -> Definition {
        Definition {
            reference: reference,
            object: object,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Stream {
    data: Vec<u8>,
}

impl Stream {
    pub fn new(data: &[u8]) -> Stream {
        Stream {
            data: data.to_vec(),
        }
    }
}

#[allow(dead_code)]
struct StreamMetadata {
    length: PdfObject,
    // TODO: the rest of the fields
}

impl StreamMetadata {
    pub fn from(mut dictionary: PdfDictionary) -> Result<StreamMetadata, String> {
        if let Some(length) = dictionary.remove("Length") {
            Ok(StreamMetadata {
                length: length,
            })
        } else {
            Err("Missing Length".to_string())
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
enum PdfObject {
    Array(Vec<PdfObject>),
    Boolean(bool),
    Reference(Reference),
    Dictionary(HashMap<String, PdfObject>),
    Float(f64),
    Identifier(String),
    Integer(i64),
    Stream(Stream),
    Null,
    String(String),
}

impl PdfObject {
    fn identifier(data: &str) -> PdfObject {
        PdfObject::Identifier(data.to_string())
    }

    fn string(data: &str) -> PdfObject {
        PdfObject::String(data.to_string())
    }

    fn reference(object: u64, generation: u64) -> PdfObject {
        PdfObject::Reference(Reference::new(object, generation))
    }
}

// 7.3.5
fn identifier<'a>(mut data: &'a [u8]) -> Res<'a, String> {
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

fn reference_header<'a>(mut data: &'a [u8]) -> Res<'a, Reference> {
    let object = block!(data, integer);
    data = consume_whitespace(data);

    let generation = block!(data, integer);

    if object < 1 || generation < 0 {
        return Res::NotFound;
    }

    Res::found(Reference::new(object as u64, generation as u64), data)
}

// 7.3.10
fn reference<'a>(mut data: &'a [u8]) -> Res<'a, Reference> {
    let reference = block!(data, reference_header);

    data = consume_whitespace(data);

    ascii!(data, ASCII_R);

    Res::found(reference, data)
}

// 7.3.10
fn definition<'a>(mut data: &'a [u8]) -> Res<'a, Definition> {
    let reference = block!(data, reference_header);
    data = consume_whitespace(data);

    exact!(data, "obj");
    data = consume_whitespace(data);

    let obj = block!(data, object);
    data = consume_whitespace(data);

    exact!(data, "endobj");

    Res::found(Definition::new(reference, obj), data)
}

// 7.3.8.1
fn stream<'a>(mut data: &'a [u8]) -> Res<'a, Stream> {
    let dict = block!(data, dictionary);

    if let Ok(_) = StreamMetadata::from(dict) {
        // TODO: use it
    } else {
        return Res::NotFound;
    }

    data = consume_whitespace(data);

    exact!(data, "stream");
    block!(data, eol);

    let mut length = 0;
    // TODO: profile and optimize this
    while data.len() > length {
        let mut local_data = &data[length..];

        // A stream is delimited by (optionally) a newline and
        // the string `endstream`
        match eol(local_data) {
            Res::Found(r) => {
                local_data = r.remaining;
            },
            _ => {}
        }

        if exact(local_data, "endstream") != Res::NotFound {
            break;
        }

        length += 1;
        continue;
    }

    let result = Stream::new(&data[0..length]);
    data = &data[length..];

    optional!(data, eol);
    exact!(data, "endstream");

    Res::found(result, data)
}

fn object<'a>(data: &'a [u8]) -> Res<'a, PdfObject> {
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
    if let Res::Found(r) = stream(data) {
        return Res::found(PdfObject::Stream(r.data), r.remaining);
    }
    if let Res::Found(r) = dictionary(data) {
        return Res::found(PdfObject::Dictionary(r.data), r.remaining);
    }

    Res::NotFound
}

/// Consumes whitespace or comments, wether they are there or not
fn consume_whitespace<'a>(mut data: &'a [u8]) -> &'a [u8] {
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
fn array<'a>(mut data: &'a [u8]) -> Res<'a, Vec<PdfObject>> {
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
fn null<'a>(data: &'a [u8]) -> Res<'a, ()> {
    exact(data, "null")
}

// 7.3.7
fn dictionary<'a>(mut data: &'a [u8]) -> Res<'a, PdfDictionary> {
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

    Res::found(result, data)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XrefType {
    Free,
    InUse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct XrefEntry {
    offset: usize,
    generation_number: u64,
    type_: XrefType,
}

impl XrefEntry {
    pub fn new(offset: usize, generation_number: u64,
               type_: XrefType) -> XrefEntry {
        XrefEntry {
            offset,
            generation_number,
            type_,
        }
    }
}

fn fixed_integer<'a>(data: &'a [u8], length: usize) -> Res<'a, u64> {
    if data.len() < length {
        return Res::NotFound;
    }

    String::from_utf8((&data[0..length]).to_vec()).ok()
        .and_then(|s| u64::from_str(&s).ok())
        .map(|x| Res::found(x, &data[length..]))
        .unwrap_or(Res::NotFound)
}

// 7.5.4
fn xref_entry<'a>(mut data: &'a [u8]) -> Res<'a, XrefEntry> {
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
    object_number: u64,
    generation_number: u64,
    type_: XrefType,
}

impl Xref {
    pub fn from(entry: XrefEntry, object_number: u64) -> Xref {
        Xref {
            offset: entry.offset,
            object_number: object_number,
            generation_number: entry.generation_number,
            type_: entry.type_,
        }
    }

    pub fn new(offset: usize, object_number: u64, generation_number: u64,
            type_: XrefType) -> Xref {
        Xref {
            offset,
            object_number,
            generation_number,
            type_,
        }
    }
}

// 7.5.4
fn xref_table<'a>(mut data: &'a [u8]) -> Res<'a, Vec<Xref>> {
    exact!(data, "xref");
    data = consume_whitespace(data);

    let mut xref_table = vec![];
    loop {
        let object_number = repeat!(data, nonnegative_integer);
        data = consume_whitespace(data);

        let entries = block!(data, nonnegative_integer);
        data = consume_whitespace(data);

        for i in 0..entries {
            let xref = block!(data, xref_entry);
            xref_table.push(Xref::from(xref, object_number as u64 + i));
        }
    }

    Res::found(xref_table, data)
}

// 7.5.5
fn startxref<'a>(mut data: &'a [u8]) -> Res<'a, u64> {
    exact!(data, "startxref");
    data = consume_whitespace(data);

    let result = block!(data, nonnegative_integer);
    Res::found(result, data)
}

// 7.5.5
fn trailer<'a>(mut data: &'a [u8]) -> Res<'a, PdfDictionary> {
    exact!(data, "trailer");
    data = consume_whitespace(data);

    let result = block!(data, dictionary);
    Res::found(result, data)
}

#[derive(Debug)]
pub struct Pdf {
    version: Version,
    objects: Vec<Definition>,
    xref_table: Vec<Xref>,
    trailer: PdfDictionary,
    startxref: u64,
}

fn pdf<'a>(mut data: &'a [u8]) -> Res<'a, Pdf> {
    let version = block!(data, version);
    data = consume_whitespace(data);

    let mut objects = vec![];
    loop {
        let object = repeat!(data, definition);
        objects.push(object);

        data = consume_whitespace(data);
    }

    let xref_table = block!(data, xref_table);
    data = consume_whitespace(data);

    let trailer = block!(data, trailer);
    data = consume_whitespace(data);

    let startxref = block!(data, startxref);

    block!(data, eol);
    block!(data, eof);

    let result = Pdf { version, objects, xref_table, trailer, startxref };
    Res::found(result, data)
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
                assert_eq!(result.data.as_str(), expected);
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

    test!(string_comment_test, string_t comment);

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
        hex_string_test("<F240D629CD72348F>",
                        "ERROR: Unparsable string.", "");
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

    test!(identifier_test, identifier);

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
            PdfObject::Dictionary(
                [
                    ("A".to_string(), PdfObject::identifier("B")),
                    ("C".to_string(), PdfObject::Array(vec![])),
                ].iter().cloned().collect()), "");
        object_test("<< /Length 8 0 R >>
            stream\n\
            BT \n\
            	/F1 12 Tf \n\
            	72 712 Td \n\
            	(A stream with an indirect length) Tj \n\
            ET\n\
            \nendstream",
            PdfObject::Stream(Stream::new(
            "BT \n\
            	/F1 12 Tf \n\
            	72 712 Td \n\
            	(A stream with an indirect length) Tj \n\
            ET\n".as_bytes())), "");
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
        let expected_map: PdfDictionary =
            expected.iter().cloned().collect();
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
            ("Subdictionary".to_string(), PdfObject::Dictionary(
                [("Item1".to_string(), PdfObject::Float(0.4)),
                 ("Item2".to_string(), PdfObject::Boolean(true)),
                 ("LastItem".to_string(), PdfObject::string("not!")),
                 ("VeryLastItem".to_string(), PdfObject::string("OK"))
                ].iter().cloned().collect()))
        ], "");
    }

    test!(reference_test, reference, Reference);

    #[test]
    fn test_reference() {
        assert_eq!(reference("0 0 R".as_bytes()), Res::NotFound);
        assert_eq!(reference("-1 0 R".as_bytes()), Res::NotFound);
        assert_eq!(reference("1 0 M".as_bytes()), Res::NotFound);

        reference_test("17 0 R", Reference::new(17, 0), "");
        reference_test("1 0 R", Reference::new(1, 0), "");
        reference_test("1 10 R", Reference::new(1, 10), "");
        reference_test("1 10 Rtest", Reference::new(1, 10), "test");
    }

    test!(definition_test, definition, Definition);

    #[test]
    fn test_definition() {
        definition_test("12 0 obj
                (Brilling)
            endobj",
            Definition::new(
                Reference::new(12, 0),
                PdfObject::string("Brilling")), "");
    }

    fn stream_test(data: &str, expected: &str, remaining: &str) {
        let result = stream(data.as_bytes()).unwrap();
        assert_eq!(from_bytes(&result.data.data[..]).as_str(), expected);
        assert_eq!(from_bytes(result.remaining).as_str(), remaining);
    }

    #[test]
    fn test_stream() {
        stream_test("<< /Length 12 >> \
            stream\n\
                123456789012\n\
            endstream", "123456789012", "");
        stream_test("<< /Length 12 >> \
            stream\n\
                123456789012endstream", "123456789012", "");
        stream_test("<< /Length 8 0 R >> % Reference length\n\
            stream\n\
                123456789012\n\
            endstream", "123456789012", "");
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

    fn fixed_integer_5<'a>(data: &'a [u8]) -> Res<'a, u64> {
        fixed_integer(data, 5)
    }

    test!(fixed_integer_5_test, fixed_integer_5, u64);

    fn fixed_integer_10<'a>(data: &'a [u8]) -> Res<'a, u64> {
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

    test!(xref_table_test, xref_table, Vec<Xref>);
    #[test]
    fn test_xref_table() {
        xref_table_test("xref", vec![], "");
        xref_table_test("xref\n\
            0 6\n\
            0000000003 65535 f\r\n\
            0000000017 00000 n\r\n\
            0000000081 00000 n\r\n\
            0000000000 00007 f\r\n\
            0000000331 00000 n\r\n\
            0000000409 00000 n\r\n",
        vec![
            Xref::new(3,   0, 65535, XrefType::Free),
            Xref::new(17,  1, 0,     XrefType::InUse),
            Xref::new(81,  2, 0,     XrefType::InUse),
            Xref::new(0,   3, 7,     XrefType::Free),
            Xref::new(331, 4, 0,     XrefType::InUse),
            Xref::new(409, 5, 0,     XrefType::InUse),
        ], "");
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
        vec![
            Xref::new(0,     0,  65535, XrefType::Free),
            Xref::new(25325, 3,  0,     XrefType::InUse),
            Xref::new(25518, 23, 2,     XrefType::InUse),
            Xref::new(25635, 24, 0,     XrefType::InUse),
            Xref::new(25777, 30, 0,     XrefType::InUse),
        ], "");
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
                    [("Root".to_string(), PdfObject::reference(1, 0)),
                     ("Size".to_string(), PdfObject::Integer(95)),
                     ("Info".to_string(), PdfObject::reference(2, 0))]
                        .iter().cloned().collect()
                , "");
    }
}
