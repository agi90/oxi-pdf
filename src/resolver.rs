use std::collections::HashMap;

use crate::parser;
use crate::parser::{
    PdfDictionary,
    OptionalFrom,
};
use crate::types::{
    NumberTreeNode,
    Rectangle,
};
use crate::font::{
    Font,
};

pub fn resolve_pdf(pdf: &parser::Pdf) -> Result<(), String> {
    for object in pdf.objects().values() {
        if let Some(dictionary) = object.as_dictionary(pdf) {
            if let Some(type_) = dictionary.identifier("Type") {
                if type_ == "Catalog" {
                    Catalog::from(dictionary, pdf).ok_or("Missing Catalog.")?;
                } else if type_ == "Font" {
                    let font : Option<Font> = OptionalFrom::from(object, pdf);
                    println!("{:#?}", font);
                }
            }
        }
    }

    Ok(())
}

// Table 28
#[allow(dead_code)]
pub struct Catalog {
    page_tree: PageTree,
    page_labels: Option<NumberTreeNode>,
}

impl Catalog {
    // 7.7.2
    fn from(metadata: &PdfDictionary,
            pdf: &parser::Pdf) -> Option<Catalog> {

        let page_tree = metadata.dictionary("Pages", pdf)
            .and_then(|pt| PageTree::from(pt, pdf))?;

        let page_labels = metadata.dictionary("PageLabels", pdf)
            .and_then(|pl| NumberTreeNode::from(pl, pdf));

        Some(Catalog{
            page_tree,
            page_labels,
        })
    }
}

#[derive(Debug, Clone)]
struct PageTree {
    kids: Vec<PageTreeNode>,
    count: usize,
}

#[derive(Debug, Clone)]
struct PageTreeNode {
    kids: Vec<PageTreeNode>,
    data: PageData,
}

impl PageTree {
    // 7.7.3.2
    pub fn from(data: &PdfDictionary,
            pdf: &parser::Pdf) -> Option<PageTree> {
        Some(PageTree {
            kids: data.map_reference_array("Kids", pdf, PageTree::kid)?,
            count: data.unsigned("Count")? as usize,
        })
    }

    fn kid(data: &PdfDictionary, pdf: &parser::Pdf) -> Option<PageTreeNode> {
        Some(PageTreeNode {
            kids: data.map_reference_array("Kids", pdf, PageTree::kid)
                      .unwrap_or(vec![]),
            data: PageData::from(data, pdf)?,
        })
    }
}

#[derive(Debug, Clone)]
struct PageData {
    media_box: Option<Rectangle>,
    resources: Option<Resources>,
}

impl PageData {
    // 7.7.3.2
    pub fn from(data: &PdfDictionary, pdf: &parser::Pdf) -> Option<PageData> {
        Some(PageData {
            media_box: data.get("MediaBox")
                .and_then(|mb| OptionalFrom::from(mb, pdf)),
            resources: data.dictionary("Resources", pdf)
                    .and_then(|r| Resources::from(r, pdf)),
        })
    }
}

// 14.2
#[derive(Debug, Clone, Copy)]
enum ProcSet {
    Pdf,
    Text,
    ImageB,
    ImageC,
    ImageI,
}

impl ProcSet {
    pub fn from(data: &str) -> Option<ProcSet> {
        match data {
            "PDF" => Some(ProcSet::Pdf),
            "Text" => Some(ProcSet::Text),
            "ImageB" => Some(ProcSet::ImageB),
            "ImageC" => Some(ProcSet::ImageC),
            "ImageI" => Some(ProcSet::ImageI),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct GraphicsState {
    line_width: Option<f64>,
    line_cap_style: Option<i64>,
    line_join_style: Option<i64>,
    miter_limit: Option<f64>,
    dash_array: Option<(Vec<u64>, u64)>,
    name: Option<String>,
    overprint: Option<bool>,
    non_stroking_overprint: Option<bool>,
    overprint_mode: Option<i64>,
}

impl GraphicsState {
    // 8.4.5
    pub fn from(data: &PdfDictionary) -> GraphicsState {
        let dash_array = data.array("D").and_then(|da| {
            if da.len() != 2 {
                return None;
            }
            Some((da[0].as_unsigned_array()?.collect(), da[1].as_unsigned()?))
        });

        let graphics_state = GraphicsState {
            line_width: data.float("LW"),
            line_cap_style: data.integer("LC"),
            line_join_style: data.integer("LJ"),
            miter_limit: data.float("ML"),
            dash_array,
            name: data.identifier("RI").map(str::to_string),
            overprint: data.boolean("OP"),
            non_stroking_overprint: data.boolean("op"),
            overprint_mode: data.integer("OPM"),
            // TODO: rest of attributes
        };

        graphics_state
    }
}

#[derive(Debug, Clone)]
struct Resources {
    proc_set: Vec<ProcSet>,
    graphics_state: Option<HashMap<String, GraphicsState>>,
}

impl Resources {
    // 7.8.3
    pub fn from(data: &PdfDictionary, pdf: &parser::Pdf) -> Option<Resources> {
        let resources = Some(Resources {
            proc_set: data.identifier_array("ProcSet")?
                    .filter_map(ProcSet::from)
                    .collect(),
            graphics_state: data.dictionary("ExtGState", pdf)
                    .map(|gs| gs.data.iter().filter_map(|(key, value)|
                        Some((key.to_string(),
                              GraphicsState::from(value.as_dictionary(pdf)?)))
                    ).collect()),
        });

        resources
    }
}
