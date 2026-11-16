//! What typed values cannot say.
//!
//! Vixen's files hold details no typed value carries: whether a colour was `Color.White` or
//! `Color.FromArgb(255, 255, 255)`, a library name that is nil rather than empty, members a newer Vixen would
//! add but this file does not have yet. Typed values stay the source of truth. At load each object is encoded
//! from its typed form and compared with what the file said; only the differences are kept, as pairs of "what
//! the file said" and "what typed encoding said at load". At save, a member whose typed encoding still matches
//! gets the file's version back, and a member that changed gets the new typed encoding.

use crate::xml::{has_significant_text, same_header, same_tree};
use std::collections::HashSet;
use vixen_xml::{XElement, XNode};

/// The differences between one object in a file and its typed encoding.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Residue {
    /// Differences that member bookkeeping cannot express. `None` means the element was not there.
    Whole { original: Option<Box<XElement>>, plain: Option<Box<XElement>> },
    /// Differences in individual members, in document order.
    Members(Vec<MemberResidue>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MemberResidue {
    local: String,
    original: Option<XElement>,
    plain: Option<XElement>,
    /// The name of the member before this one in the file, to place a member typed encoding does not write.
    after: Option<String>,
}

impl MemberResidue {
    pub(crate) fn name(&self) -> &str {
        &self.local
    }
}

/// Compares an object with its typed encoding. `None` means typed encoding reproduces it exactly.
pub(crate) fn diff(original: &XElement, plain: &XElement) -> Option<Residue> {
    if same_tree(original, plain) {
        return None;
    }
    let whole = || Residue::Whole { original: Some(Box::new(original.clone())), plain: Some(Box::new(plain.clone())) };
    if !same_header(original, plain)
        || has_significant_text(original)
        || has_significant_text(plain)
        || has_repeated_names(original)
        || has_repeated_names(plain)
    {
        return Some(whole());
    }
    let mut members = Vec::new();
    let mut after: Option<&str> = None;
    for o in original.elements() {
        let p = plain.child(&o.local);
        if !p.is_some_and(|p| same_tree(o, p)) {
            members.push(MemberResidue {
                local: o.local.clone(),
                original: Some(o.clone()),
                plain: p.cloned(),
                after: after.map(str::to_string),
            });
        }
        after = Some(&o.local);
    }
    for p in plain.elements() {
        if original.child(&p.local).is_none() {
            members.push(MemberResidue { local: p.local.clone(), original: None, plain: Some(p.clone()), after: None });
        }
    }
    let residue = Residue::Members(members);
    // Member bookkeeping cannot fix a different member order, so check it really reproduces the original.
    if same_tree(&reapply(plain.clone(), Some(&residue)), original) {
        Some(residue)
    } else {
        Some(whole())
    }
}

/// [`diff`] for an element that may be missing on either side.
pub(crate) fn diff_opt(original: Option<&XElement>, plain: Option<&XElement>) -> Option<Residue> {
    match (original, plain) {
        (None, None) => None,
        (Some(o), Some(p)) => diff(o, p),
        (o, p) => Some(Residue::Whole { original: o.cloned().map(Box::new), plain: p.cloned().map(Box::new) }),
    }
}

/// Restores what the file said, for every member whose typed encoding has not changed since load.
pub(crate) fn reapply(plain: XElement, residue: Option<&Residue>) -> XElement {
    match residue {
        None => plain,
        Some(Residue::Whole { original: Some(original), plain: Some(at_load) }) => {
            if same_tree(&plain, at_load) {
                (**original).clone()
            } else {
                plain
            }
        }
        Some(Residue::Whole { .. }) => plain,
        Some(Residue::Members(members)) => apply_members(plain, members),
    }
}

/// [`reapply`] for an element typed encoding may not produce, or that the file did not have.
pub(crate) fn reapply_opt(plain: Option<XElement>, residue: Option<&Residue>) -> Option<XElement> {
    match (plain, residue) {
        (plain, Some(Residue::Whole { original, plain: at_load })) => {
            let unchanged = match (&plain, at_load) {
                (None, None) => true,
                (Some(p), Some(q)) => same_tree(p, q),
                _ => false,
            };
            if unchanged {
                original.as_deref().cloned()
            } else {
                plain
            }
        }
        (Some(plain), residue) => Some(reapply(plain, residue)),
        (None, _) => None,
    }
}

fn apply_members(mut plain: XElement, members: &[MemberResidue]) -> XElement {
    for m in members {
        let position = plain.children.iter().position(|n| matches!(n, XNode::Element(e) if e.local == m.local));
        let current = position.and_then(|p| match &plain.children[p] {
            XNode::Element(e) => Some(e),
            XNode::Text(_) => None,
        });
        let unchanged = match (current, &m.plain) {
            (None, None) => true,
            (Some(c), Some(p)) => same_tree(c, p),
            _ => false,
        };
        if !unchanged {
            continue;
        }
        match (position, &m.original) {
            (Some(p), Some(o)) => plain.children[p] = XNode::Element(o.clone()),
            (Some(p), None) => {
                plain.children.remove(p);
            }
            (None, Some(o)) => {
                let at = m
                    .after
                    .as_ref()
                    .and_then(|a| plain.children.iter().position(|n| matches!(n, XNode::Element(e) if e.local == *a)))
                    .map_or(0, |i| i + 1);
                plain.children.insert(at, XNode::Element(o.clone()));
            }
            (None, None) => {}
        }
    }
    plain
}

fn has_repeated_names(e: &XElement) -> bool {
    let mut seen = HashSet::new();
    e.elements().any(|c| !seen.insert(c.local.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(t: &str) -> XElement {
        XElement::parse(t).unwrap()
    }

    #[test]
    fn identical_objects_have_no_residue() {
        let a = parse("<o><a>1</a><b>2</b></o>");
        assert_eq!(diff(&a, &a.clone()), None);
    }

    #[test]
    fn a_changed_member_comes_back_when_typed_encoding_is_unchanged() {
        let file = parse("<o><a>01</a><b>2</b></o>");
        let plain = parse("<o><a>1</a><b>2</b></o>");
        let r = diff(&file, &plain).unwrap();
        assert!(same_tree(&reapply(plain.clone(), Some(&r)), &file));
    }

    #[test]
    fn an_edit_replaces_only_the_member_it_touches() {
        let file = parse("<o><a>01</a><b>2</b></o>");
        let plain = parse("<o><a>1</a><b>2</b></o>");
        let r = diff(&file, &plain).unwrap();
        let edited = parse("<o><a>5</a><b>3</b></o>");
        assert!(same_tree(&reapply(edited.clone(), Some(&r)), &edited));
        let other = parse("<o><a>1</a><b>3</b></o>");
        assert!(same_tree(&reapply(other, Some(&r)), &parse("<o><a>01</a><b>3</b></o>")));
    }

    #[test]
    fn members_typed_encoding_does_not_know_stay_in_place() {
        let file = parse("<o><a>1</a><future>x</future><b>2</b></o>");
        let plain = parse("<o><a>1</a><b>2</b></o>");
        let r = diff(&file, &plain).unwrap();
        assert!(same_tree(&reapply(plain, Some(&r)), &file));
        let edited = parse("<o><a>9</a><b>2</b></o>");
        assert!(same_tree(&reapply(edited, Some(&r)), &parse("<o><a>9</a><future>x</future><b>2</b></o>")));
    }

    #[test]
    fn members_the_file_lacks_stay_absent_until_they_change() {
        let file = parse("<o><a>1</a></o>");
        let plain = parse("<o><a>1</a><added>0</added></o>");
        let r = diff(&file, &plain).unwrap();
        assert!(same_tree(&reapply(plain, Some(&r)), &file));
        let edited = parse("<o><a>1</a><added>7</added></o>");
        assert!(same_tree(&reapply(edited.clone(), Some(&r)), &edited));
    }

    #[test]
    fn a_different_member_order_falls_back_to_the_whole_object() {
        let file = parse("<o><b>2</b><a>1</a></o>");
        let plain = parse("<o><a>1</a><b>2</b></o>");
        let r = diff(&file, &plain).unwrap();
        assert!(matches!(r, Residue::Whole { .. }));
        assert!(same_tree(&reapply(plain, Some(&r)), &file));
    }

    #[test]
    fn lists_and_text_fall_back_to_the_whole_object() {
        let file = parse("<o><i>1</i><i>2</i></o>");
        let plain = parse("<o><i>1</i></o>");
        assert!(matches!(diff(&file, &plain), Some(Residue::Whole { .. })));
        let file = parse("<o>01</o>");
        let plain = parse("<o>1</o>");
        let r = diff(&file, &plain).unwrap();
        assert!(same_tree(&reapply(plain, Some(&r)), &file));
    }

    #[test]
    fn optional_elements() {
        let e = parse("<o/>");
        let r = diff_opt(None, Some(&e)).unwrap();
        assert_eq!(reapply_opt(Some(e.clone()), Some(&r)), None);
        assert_eq!(reapply_opt(Some(parse("<o><x/></o>")), Some(&r)), Some(parse("<o><x/></o>")));
        let r = diff_opt(Some(&e), None).unwrap();
        assert_eq!(reapply_opt(None, Some(&r)), Some(e.clone()));
        assert_eq!(diff_opt(None, None), None);
    }
}
