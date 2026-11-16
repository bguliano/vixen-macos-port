//! xLights custom/circle model import, matching the upstream XLights parsers and prop assembler.
mod assemble;
mod children;
mod colors;
mod geometry;
#[cfg(test)]
mod tests;

use crate::{Prop, PropError};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use vixen_xml::XElement;

const MAX_NODES: usize = 1_000_000;
const MAX_XML_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCandidate {
    pub index: usize,
    pub display_name: String,
    pub model_type: String,
    pub supported: bool,
}

#[derive(Clone, Debug)]
pub struct XModelFile {
    models: Vec<XElement>,
    candidates: Vec<ModelCandidate>,
}
#[derive(Debug)]
pub struct ImportResult {
    pub prop: Prop,
    pub warnings: Vec<String>,
}

impl XModelFile {
    pub fn open(path: &Path) -> Result<Self, PropError> {
        if std::fs::metadata(path)
            .map_err(|e| failed(e.to_string()))?
            .len()
            > MAX_XML_BYTES as u64
        {
            return Err(failed("The xModel file is too large."));
        }
        let bytes = std::fs::read(path).map_err(|e| failed(e.to_string()))?;
        let text = crate::vendor::decode_text(&bytes).map_err(failed)?;
        Self::parse(&text)
    }
    pub fn parse(text: &str) -> Result<Self, PropError> {
        if text.len() > MAX_XML_BYTES {
            return Err(failed("The xModel file is too large."));
        }
        let root = XElement::parse(text).map_err(|e| failed(e.to_string()))?;
        let models = if kind(&root).is_some() {
            vec![root]
        } else if root.local == "models" {
            root.elements()
                .filter(|e| e.local == "model")
                .cloned()
                .collect()
        } else {
            return Err(unsupported(&model_type(&root)));
        };
        if models.is_empty() {
            return Err(failed("No models were found in the xModel file."));
        }
        let mut candidates: Vec<_> = models
            .iter()
            .enumerate()
            .map(|(index, e)| ModelCandidate {
                index,
                display_name: if attr(e, "name").trim().is_empty() {
                    format!("Unnamed model {}", index + 1)
                } else {
                    attr(e, "name").trim().into()
                },
                model_type: model_type(e),
                supported: kind(e).is_some(),
            })
            .collect();
        if candidates.iter().all(|c| !c.supported) {
            return Err(unsupported(&candidates[0].model_type));
        }
        let mut totals = HashMap::new();
        let mut counts = HashMap::new();
        for c in &candidates {
            *totals.entry(c.display_name.clone()).or_insert(0) += 1;
        }
        for c in &mut candidates {
            if totals[&c.display_name] > 1 {
                let n = counts.entry(c.display_name.clone()).or_insert(0);
                *n += 1;
                c.display_name = format!("{} ({n})", c.display_name);
            }
        }
        Ok(Self { models, candidates })
    }
    pub fn candidates(&self) -> &[ModelCandidate] {
        &self.candidates
    }
    pub fn import(&self, index: usize, now: &str) -> Result<ImportResult, PropError> {
        let e = self
            .models
            .get(index)
            .ok_or_else(|| failed("The selected model no longer exists."))?;
        let kind = kind(e).ok_or_else(|| unsupported(&model_type(e)))?;
        let mut warnings = Vec::new();
        let pixel_size = positive(attr(e, "PixelSize")).unwrap_or_else(|| {
            if !attr(e, "PixelSize").trim().is_empty() {
                warnings.push("Invalid PixelSize; using 1.".into());
            }
            1
        });
        let geometry = geometry::parse(e, kind, &mut warnings)?;
        let children = children::parse(e)?;
        let prop = assemble::assemble(attr(e, "name"), pixel_size, geometry, children, now)?;
        Ok(ImportResult { prop, warnings })
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Custom,
    Circle,
}
struct Geometry {
    nodes: BTreeMap<i32, (i32, i32)>,
    width: i32,
    height: i32,
    rings: Vec<Vec<i32>>,
}
fn attr<'a>(e: &'a XElement, key: &str) -> &'a str {
    // XElement.Attribute(name) requires an unqualified attribute, even when the element has a namespace.
    e.attrs
        .iter()
        .find(|a| a.ns.is_empty() && a.local == key)
        .map(|a| a.value.as_str())
        .unwrap_or("")
}
fn kind(e: &XElement) -> Option<Kind> {
    if e.attrs.is_empty() {
        return None;
    }
    match e.local.as_str() {
        "custommodel" => Some(Kind::Custom),
        "circlemodel" => Some(Kind::Circle),
        "model" if attr(e, "DisplayAs").eq_ignore_ascii_case("Custom") => Some(Kind::Custom),
        "model" if attr(e, "DisplayAs").eq_ignore_ascii_case("Circle") => Some(Kind::Circle),
        _ => None,
    }
}
fn model_type(e: &XElement) -> String {
    if e.local == "model" && !attr(e, "DisplayAs").trim().is_empty() {
        format!("{}model", attr(e, "DisplayAs").to_lowercase())
    } else {
        e.local.clone()
    }
}
fn integer(s: &str) -> Option<i32> {
    s.trim().parse().ok()
}
fn positive(s: &str) -> Option<i32> {
    integer(s).filter(|v| *v > 0)
}
fn failed(detail: impl Into<String>) -> PropError {
    PropError::new("Model import error", detail)
}
fn unsupported(kind: &str) -> PropError {
    failed(format!("Unsupported model type: {kind}. \nImport supports custom and circle model types at this time."))
}
