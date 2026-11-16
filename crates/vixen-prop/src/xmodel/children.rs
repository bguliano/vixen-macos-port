use super::*;
use crate::{FaceComponent, FaceDefinition};
use vixen_core::Color;

pub(super) type Ranges = Vec<(i32, i32)>;
pub(super) struct Children {
    pub subs: Vec<(String, Vec<Ranges>)>,
    pub faces: Vec<(String, Vec<Face>)>,
    pub states: Vec<State>,
}
pub(super) struct Face {
    pub name: String,
    pub ranges: Ranges,
    pub definition: FaceDefinition,
}
pub(super) struct State {
    pub name: String,
    pub items: Vec<StateItem>,
}
pub(super) struct StateItem {
    pub index: i32,
    pub name: String,
    pub color: Color,
    pub ranges: Option<Ranges>,
}

pub(super) fn parse(e: &XElement) -> Result<Children, PropError> {
    let mut out = Children {
        subs: vec![],
        faces: vec![],
        states: vec![],
    };
    for child in e.elements() {
        match child.local.as_str() {
            "subModel" if attr(child, "type") == "ranges" => {
                let mut lines = Vec::new();
                let mut n = 0;
                loop {
                    let value = attr(child, &format!("line{n}"));
                    if value.is_empty() {
                        break;
                    }
                    lines.push(ranges(value)?);
                    n += 1;
                }
                out.subs.push((attr(child, "name").into(), lines));
            }
            "faceInfo" if attr(child, "Type") == "NodeRange" => {
                let mut items = Vec::new();
                for (key, component) in FACE_ATTRS {
                    let value = attr(child, key);
                    if value.is_empty() {
                        continue;
                    }
                    let color_text = attr(child, &format!("{key}-Color"));
                    let mut color = colors::parse(color_text)?;
                    // FromHtml numeric black has Name="ff000000"; named Black keeps its name.
                    if color == Color::BLACK && colors::is_numeric(color_text) {
                        color = Color::WHITE;
                    }
                    items.push(Face {
                        name: (*key).into(),
                        ranges: ranges(value)?,
                        definition: FaceDefinition {
                            component: *component,
                            default_color: color,
                        },
                    });
                }
                out.faces.push((attr(child, "Name").into(), items));
            }
            "stateInfo" if matches!(attr(child, "Type"), "NodeRange" | "SingleNode") => {
                let name = attr(child, "Name").trim();
                let name = if name.is_empty() {
                    format!("State Name {}", out.states.len() + 1)
                } else {
                    name.into()
                };
                let mut items: Vec<(String, StateItem)> = Vec::new();
                for a in &child.attrs {
                    if !a.local.starts_with('s') {
                        continue;
                    }
                    let parts: Vec<_> = a.local.split('-').collect();
                    let tag = parts[0];
                    let Some(index) = integer(&tag[1..]) else {
                        continue;
                    };
                    let pos = items
                        .iter()
                        .position(|(key, _)| key == tag)
                        .unwrap_or_else(|| {
                            items.push((
                                tag.into(),
                                StateItem {
                                    index,
                                    name: tag.into(),
                                    color: Color::WHITE,
                                    ranges: None,
                                },
                            ));
                            items.len() - 1
                        });
                    let item = &mut items[pos].1;
                    match parts.as_slice() {
                        [_] if attr(child, "Type") == "NodeRange" => {
                            item.ranges = Some(ranges(&a.value)?)
                        }
                        [_] => {
                            let tokens: Vec<_> = a.value.split(' ').collect();
                            if tokens.len() == 2 && integer(tokens[1]).is_some() {
                                item.ranges = Some(ranges(tokens[1])?);
                            }
                        }
                        [_, "Color"] if !a.value.is_empty() => {
                            item.color = colors::parse(&a.value)?
                        }
                        [_, "Name"] => {
                            item.name = if a.value.trim().is_empty() {
                                tag.into()
                            } else {
                                a.value.trim().into()
                            }
                        }
                        _ => {}
                    }
                }
                out.states.push(State {
                    name,
                    items: items
                        .into_iter()
                        .map(|(_, v)| v)
                        .filter(|v| v.ranges.is_some())
                        .collect(),
                });
            }
            _ => {}
        }
    }
    Ok(out)
}
fn ranges(value: &str) -> Result<Ranges, PropError> {
    let mut out = Vec::new();
    for token in value.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let parts: Vec<_> = token.split('-').collect();
        let bad = || failed(format!("Invalid node range: {token}"));
        match parts.as_slice() {
            [v] => {
                let n = integer(v).ok_or_else(bad)?;
                out.push((n, n));
            }
            [a, b] => out.push((integer(a).ok_or_else(bad)?, integer(b).ok_or_else(bad)?)),
            _ => {}
        }
    }
    Ok(out)
}
const FACE_ATTRS: &[(&str, FaceComponent)] = &[
    ("Eyes-Closed", FaceComponent::EyesClosed),
    ("Eyes-Open", FaceComponent::EyesOpen),
    ("Eyes-Closed2", FaceComponent::EyesClosed2),
    ("Eyes-Open2", FaceComponent::EyesOpen2),
    ("Eyes-Closed3", FaceComponent::EyesClosed3),
    ("Eyes-Open3", FaceComponent::EyesOpen3),
    ("FaceOutline", FaceComponent::Outlines),
    ("FaceOutline2", FaceComponent::Outlines),
    ("Mouth-AI", FaceComponent::AI),
    ("Mouth-FV", FaceComponent::FV),
    ("Mouth-MBP", FaceComponent::MBP),
    ("Mouth-E", FaceComponent::E),
    ("Mouth-L", FaceComponent::L),
    ("Mouth-O", FaceComponent::O),
    ("Mouth-U", FaceComponent::U),
    ("Mouth-WQ", FaceComponent::WQ),
    ("Mouth-etc", FaceComponent::ETC),
    ("Mouth-rest", FaceComponent::REST),
];
