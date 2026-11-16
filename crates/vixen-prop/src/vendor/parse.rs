use super::*;
use std::collections::HashMap;
use vixen_xml::XElement;

pub fn parse_vendor_list(text: &str) -> Result<Vec<Link>, String> {
    let json: serde_json::Value =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    json.as_array()
        .ok_or("Vendor list must be an array.")?
        .iter()
        .map(|v| {
            Ok(Link {
                name: v
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into(),
                url: v
                    .get("Url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into(),
            })
        })
        .collect()
}
// `new Uri(string)` expects an absolute URI. Native networking performs the final protocol validation.
pub(super) fn absolute_uri(value: &str) -> Option<String> {
    let value = value.trim();
    let (scheme, tail) = value.split_once(':')?;
    if scheme.is_empty()
        || !scheme.as_bytes()[0].is_ascii_alphabetic()
        || !scheme
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
        || tail.is_empty()
        || value.chars().any(char::is_control)
    {
        return None;
    }
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        let authority = tail.strip_prefix("//")?.split(['/', '?', '#']).next()?;
        if authority.is_empty() || authority.chars().any(char::is_whitespace) {
            return None;
        }
    }
    Some(value.into())
}
fn child<'a>(e: &'a XElement, name: &str) -> Option<&'a XElement> {
    e.elements().find(|c| c.ns.is_empty() && c.local == name)
}
fn elements<'a>(e: &'a XElement, name: &'a str) -> impl Iterator<Item = &'a XElement> {
    e.elements()
        .filter(move |c| c.ns.is_empty() && c.local == name)
}
fn text(e: &XElement, name: &str) -> String {
    child(e, name).map(|c| c.text()).unwrap_or_default()
}
fn uri(e: &XElement, name: &str) -> Result<Option<String>, String> {
    let value = text(e, name);
    if value.is_empty() {
        Ok(None)
    } else {
        absolute_uri(&value)
            .map(Some)
            .ok_or_else(|| format!("Invalid {name} URL: {value}"))
    }
}
fn uint(e: &XElement, name: &str) -> Result<u32, String> {
    let value = text(e, name);
    if child(e, name).is_none() {
        return Ok(0);
    }
    value
        .trim()
        .parse()
        .map_err(|_| format!("Invalid {name}: {value}"))
}
pub fn parse_inventory(xml: &str) -> Result<Inventory, String> {
    if xml.len() > MAX_DOWNLOAD_BYTES {
        return Err("The inventory is too large.".into());
    }
    let root = XElement::parse(xml).map_err(|e| e.to_string())?;
    if root.local != "modelinventory" || !root.ns.is_empty() {
        return Err("Expected modelinventory root.".into());
    }
    let vendor = child(&root, "vendor").ok_or("Inventory has no vendor.")?;
    let models = child(&root, "models").ok_or("Inventory has no models.")?;
    let categories = child(&root, "categories").ok_or("Inventory has no categories.")?;
    let mut links = Vec::new();
    for (key, label) in [("website", "Website"), ("facebook", "Facebook")] {
        if let Some(url) = uri(vendor, key)? {
            links.push(Link {
                name: label.into(),
                url,
            });
        }
    }
    let vendor = Vendor {
        name: text(vendor, "name"),
        contact: text(vendor, "contact"),
        email: text(vendor, "email"),
        phone: text(vendor, "phone"),
        description: text(vendor, "notes"),
        logo: uri(vendor, "logolink")?,
        links,
    };
    let mut products = Vec::new();
    let mut product_map: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut warnings = Vec::new();
    for m in elements(models, "model") {
        let id = text(m, "id").trim().parse::<u32>().unwrap_or_else(|_| {
            warnings.push(format!(
                "Unable to parse id for model {}, {}",
                text(m, "id"),
                text(m, "name")
            ));
            0
        });
        let category_ids: Vec<u32> = elements(m, "categoryid")
            .map(|c| {
                c.text()
                    .trim()
                    .parse()
                    .map_err(|_| "Invalid categoryid.".to_string())
            })
            .collect::<Result<_, _>>()?;
        let image = uri(m, "imagefile")?;
        let mut model_links = Vec::new();
        let mut unnamed = 1;
        for wiring in elements(m, "wiring") {
            let link = text(wiring, "xmodellink");
            if link.is_empty() {
                continue;
            }
            let url =
                absolute_uri(&link).ok_or_else(|| format!("Invalid xmodellink URL: {link}"))?;
            let mut name = text(wiring, "name");
            if name.is_empty() {
                name = format!("Model {unnamed}");
                unnamed += 1;
            }
            let description = text(wiring, "description");
            let description = if description.is_empty() {
                "No description provided.".into()
            } else {
                description
            };
            model_links.push(ModelLink {
                kind: if link.ends_with(".prp") {
                    ModelType::Prop
                } else {
                    ModelType::XModel
                },
                name,
                description,
                url,
            });
        }
        let mut weblink = text(m, "weblink");
        let url = if weblink.is_empty() {
            None
        } else {
            if !weblink.trim_start().to_lowercase().starts_with("http") {
                weblink = format!("http://{}", weblink.trim_start());
            }
            let parsed = absolute_uri(&weblink);
            if parsed.is_none() {
                warnings.push(format!("Invalid product weblink: {weblink}"));
            }
            parsed
        };
        let product = Product {
            id,
            category_ids,
            name: text(m, "name"),
            product_type: text(m, "type"),
            url,
            image,
            material: text(m, "material"),
            width: text(m, "width"),
            height: text(m, "height"),
            thickness: text(m, "thickness"),
            pixel_count: text(m, "pixelcount").trim().parse().unwrap_or(0),
            pixel_description: text(m, "pixeldescription"),
            pixel_spacing: text(m, "pixelspacing"),
            notes: text(m, "notes"),
            models: model_links,
        };
        let index = products.len();
        for id in &product.category_ids {
            product_map.entry(*id).or_default().push(index);
        }
        products.push(product);
    }
    fn read_categories(
        e: &XElement,
        map: &HashMap<u32, Vec<usize>>,
        depth: usize,
    ) -> Result<Vec<Category>, String> {
        if depth > 128 {
            return Err("Inventory category nesting is too deep.".into());
        }
        elements(e, "category")
            .map(|c| {
                let id = uint(c, "id")?;
                Ok(Category {
                    id,
                    name: text(c, "name"),
                    categories: match child(c, "categories") {
                        Some(kids) => read_categories(kids, map, depth + 1)?,
                        None => vec![],
                    },
                    products: map.get(&id).cloned().unwrap_or_default(),
                })
            })
            .collect()
    }
    Ok(Inventory {
        vendor,
        categories: read_categories(categories, &product_map, 0)?,
        products,
        warnings,
    })
}
