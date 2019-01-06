use std::collections::HashMap;

use crate::parser;
use crate::parser::{
    PdfObject,
    PdfDictionary,
    OptionalFrom,
};

// 7.9.5
#[derive(Debug, Clone)]
pub struct Rectangle {
    ll_x: f64,
    ll_y: f64,
    ur_x: f64,
    ur_y: f64,
}

impl OptionalFrom for Rectangle {
    fn from(obj: &PdfObject, _: &parser::Pdf) -> Option<Rectangle> {
        let mut data = obj.as_float_array()?;
        Some(Rectangle {
            ll_x: data.next()?,
            ll_y: data.next()?,
            ur_x: data.next()?,
            ur_y: data.next()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NumberTreeNode {
    kids: Vec<NumberTreeNode>,
    nums: HashMap<i64, PdfObject>,
    upper_limit: Option<i64>,
    lower_limit: Option<i64>,
}

impl NumberTreeNode {
    // 7.9.7
    pub fn from(data: &PdfDictionary, pdf: &parser::Pdf) -> Option<NumberTreeNode> {
        let nums_array = data.array("Nums").unwrap_or(&[]);

        if nums_array.len() % 2 != 0 {
            return None;
        }

        let mut nums = HashMap::new();
        for i in 0..nums_array.len() / 2 {
            let index = 2 * i;
            nums.insert(
                nums_array[index].as_integer()?,
                pdf.resolve(nums_array[index + 1].as_reference()?).clone());
        }

        let (upper_limit, lower_limit) =
                if let Some(mut limits) = data.integer_array("Limits") {
            (limits.next(), limits.next())
        } else {
            (None, None)
        };

        Some(NumberTreeNode {
            kids: data.map_reference_array("Kids", pdf, NumberTreeNode::from)
                    .unwrap_or(vec![]),
            nums,
            upper_limit,
            lower_limit,
        })
    }
}
