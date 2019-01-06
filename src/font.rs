use crate::parser;
use crate::parser::{
    PdfObject,
    OptionalFrom,
};
use crate::types::{
    Rectangle,
};

#[derive(Debug, Clone)]
pub enum Font {
    Type1(Type1Font),
}

impl OptionalFrom for Font {
    fn from(data: &PdfObject, pdf: &parser::Pdf) -> Option<Font> {
        match data.as_dictionary(pdf)?.identifier("Subtype")? {
            "Type1" => Some(Font::Type1(OptionalFrom::from(data, pdf)?)),
            _ => None,
        }
    }
}

pdf_struct!{
    Type1Font {
        required: [
            base_font: String, "BaseFont",
            first_char: i64, "FirstChar",
            last_char: i64, "LastChar",
            widths: Vec<i64>, "Widths"
        ],
        optional: [
            descriptor: FontDescriptor, "FontDescriptor"
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl OptionalFrom for FontStretch {
    // 9.6.2
    fn from(data: &PdfObject, _: &parser::Pdf) -> Option<FontStretch> {
        match data.as_identifier()? {
            "UltraCondensed" => Some(FontStretch::UltraCondensed),
            "ExtraCondensed" => Some(FontStretch::ExtraCondensed),
            "Condensed" => Some(FontStretch::Condensed),
            "SemiCondensed" => Some(FontStretch::SemiCondensed),
            "Normal" => Some(FontStretch::Normal),
            "SemiExpanded" => Some(FontStretch::SemiExpanded),
            "Expanded" => Some(FontStretch::Expanded),
            "ExtraExpanded" => Some(FontStretch::ExtraExpanded),
            "UltraExpanded" => Some(FontStretch::UltraExpanded),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FontFlag {
    FixedPitch,
    Serif,
    Symbolic,
    Script,
    Nonsymbolic,
    Italic,
    AllCap,
    SmallCap,
    ForceBold,
}

fn has_bit(data: u64, i: u32) -> bool {
    (data & (1 << (i - 1))) > 0
}

impl OptionalFrom for Vec<FontFlag> {
    // 9.8.2
    fn from(obj: &PdfObject, _: &parser::Pdf) -> Option<Vec<FontFlag>> {
        let data = obj.as_unsigned()?;
        let mut flags = vec![];
        if has_bit(data,  1) { flags.push(FontFlag::FixedPitch); }
        if has_bit(data,  2) { flags.push(FontFlag::Serif); }
        if has_bit(data,  3) { flags.push(FontFlag::Symbolic); }
        if has_bit(data,  4) { flags.push(FontFlag::Script); }
        if has_bit(data,  6) { flags.push(FontFlag::Nonsymbolic); }
        if has_bit(data,  7) { flags.push(FontFlag::Italic); }
        if has_bit(data, 17) { flags.push(FontFlag::AllCap); }
        if has_bit(data, 18) { flags.push(FontFlag::SmallCap); }
        if has_bit(data, 19) { flags.push(FontFlag::ForceBold); }
        Some(flags)
    }
}

pdf_struct!{
    FontDescriptor {
        required: [
            weight: u64, "FontWeight",
            flags: Vec<FontFlag>, "Flags",
            bounding_box: Rectangle, "FontBBox",
            italic_angle: f64, "ItalicAngle",
            ascent: f64, "Ascent",
            descent: f64, "Descent"
        ],
        optional: [
            stretch: FontStretch, "FontStretch",
            leading: f64, "Leading",
            cap_height: f64, "CapHeight",
            x_height: f64, "XHeight",
            stem_vertical: f64, "StemV",
            stem_horizontal: f64, "StemH",
            average_width: f64, "AvgWidth",
            max_width: f64, "MaxWidth",
            missing_width: f64, "MissingWidth",
            char_set: Vec<u8>, "CharSet"
        ]
    }
}
