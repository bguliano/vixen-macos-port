//! Custom Prop Vendor Browser: inventory parsing, navigation and imported metadata.
//! Transport is injected so tests never connect to a vendor.
mod parse;
#[cfg(test)]
mod tests;
use crate::{Prop, PropError};

pub const VENDOR_URL: &str = "https://app.vixenlights.com/vendor.json";
pub const MAX_DOWNLOAD_BYTES: usize = 128 * 1024 * 1024;
pub trait Transport: Send + Sync {
    fn get(&self, url: &str) -> Result<Vec<u8>, String>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub name: String,
    pub url: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vendor {
    pub name: String,
    pub contact: String,
    pub email: String,
    pub phone: String,
    pub description: String,
    pub logo: Option<String>,
    pub links: Vec<Link>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelType {
    Prop,
    XModel,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelLink {
    pub kind: ModelType,
    pub name: String,
    pub description: String,
    pub url: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Product {
    pub id: u32,
    pub category_ids: Vec<u32>,
    pub name: String,
    pub product_type: String,
    pub url: Option<String>,
    pub image: Option<String>,
    pub material: String,
    pub width: String,
    pub height: String,
    pub thickness: String,
    pub pixel_count: u32,
    pub pixel_description: String,
    pub pixel_spacing: String,
    pub notes: String,
    pub models: Vec<ModelLink>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub categories: Vec<Category>,
    pub products: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub vendor: Vendor,
    pub categories: Vec<Category>,
    pub products: Vec<Product>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryError {
    pub title: String,
    pub message: String,
    pub detail: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub path: String,
    pub name: String,
    pub product: bool,
    pub child_count: usize,
}
#[derive(Clone, Debug)]
pub struct Selection {
    pub vendor: Vendor,
    pub product: Product,
    pub model: ModelLink,
}
#[derive(Default)]
pub struct Browser {
    pub inventories: Vec<Inventory>,
    pub vendor: Option<usize>,
    pub product: Option<usize>,
}

pub use parse::{parse_inventory, parse_vendor_list};
pub fn decode_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err("The downloaded file is too large.".into());
    }
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if bytes.len() % 2 != 0 {
            return Err("Invalid UTF-16 text.".into());
        }
        let little = bytes[0] == 0xff;
        let words: Vec<_> = bytes[2..]
            .chunks_exact(2)
            .map(|p| {
                if little {
                    u16::from_le_bytes([p[0], p[1]])
                } else {
                    u16::from_be_bytes([p[0], p[1]])
                }
            })
            .collect();
        String::from_utf16(&words).map_err(|e| e.to_string())
    } else {
        String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())
    }
}
pub fn load(
    transport: &dyn Transport,
    url: &str,
    mut status: impl FnMut(String),
    mut on_error: impl FnMut(InventoryError),
) -> Vec<Inventory> {
    status("Loading vendor list...".into());
    let vendors = match transport
        .get(url)
        .and_then(|b| decode_text(&b))
        .and_then(|s| parse_vendor_list(&s))
    {
        Ok(v) => v,
        Err(detail) => {
            on_error(InventoryError {
                title: "Error Retrieving Vendors".into(),
                message:
                    "Unable to retrieve vendor list. Ensure you have an active internet connection."
                        .into(),
                detail,
            });
            status(String::new());
            return vec![];
        }
    };
    let mut inventories = Vec::new();
    for vendor in vendors {
        status(format!("Retrieving inventory from {}...", vendor.name));
        match parse::absolute_uri(&vendor.url).ok_or_else(||"Invalid inventory URL.".into()).and_then(|u|transport.get(&u)).and_then(|b|decode_text(&b)).and_then(|s|parse_inventory(&s)) {
            Ok(inventory)=>inventories.push(inventory),Err(detail)=>on_error(InventoryError {title:"Error Retrieving Inventory".into(),message:format!("Unable to retrieve inventory from {}\nEnsure you have an active internet connection.",vendor.name),detail}),
        }
    }
    status(String::new());
    inventories
}
impl Browser {
    pub fn set_inventories(&mut self, inventories: Vec<Inventory>) {
        self.vendor = if inventories.is_empty() {
            None
        } else {
            Some(0)
        };
        self.product = None;
        self.inventories = inventories;
    }
    pub fn select_vendor(&mut self, index: usize) {
        if index < self.inventories.len() {
            self.vendor = Some(index);
            self.product = None;
        }
    }
    pub fn inventory(&self) -> Option<&Inventory> {
        self.inventories.get(self.vendor?)
    }
    pub fn selected_product(&self) -> Option<&Product> {
        self.inventory()?.products.get(self.product?)
    }
    fn category(&self, path: &str) -> Option<&Category> {
        let mut rows = &self.inventory()?.categories;
        let mut found = None;
        for part in path.split('/') {
            let index = part.strip_prefix('c')?.parse::<usize>().ok()?;
            let row = rows.get(index)?;
            found = Some(row);
            rows = &row.categories;
        }
        found
    }
    pub fn rows(&self, parent: Option<&str>) -> Vec<TreeRow> {
        let Some(inventory) = self.inventory() else {
            return vec![];
        };
        let parent = parent.unwrap_or("");
        let (categories, products) = if parent.is_empty() {
            (&inventory.categories[..], &[][..])
        } else {
            match self.category(parent) {
                Some(c) => (&c.categories[..], &c.products[..]),
                None => return vec![],
            }
        };
        let prefix = if parent.is_empty() {
            String::new()
        } else {
            format!("{parent}/")
        };
        categories
            .iter()
            .enumerate()
            .map(|(i, c)| TreeRow {
                path: format!("{prefix}c{i}"),
                name: c.name.clone(),
                product: false,
                child_count: c.categories.len() + c.products.len(),
            })
            .chain(products.iter().enumerate().filter_map(|(i, id)| {
                inventory.products.get(*id).map(|p| TreeRow {
                    path: format!("{prefix}p{i}"),
                    name: p.name.clone(),
                    product: true,
                    child_count: 0,
                })
            }))
            .collect()
    }
    pub fn select_product(&mut self, path: &str) -> bool {
        let Some((parent, part)) = path.rsplit_once('/') else {
            return false;
        };
        let Some(index) = part.strip_prefix('p').and_then(|v| v.parse::<usize>().ok()) else {
            return false;
        };
        let Some(id) = self
            .category(parent)
            .and_then(|c| c.products.get(index))
            .copied()
        else {
            return false;
        };
        self.product = Some(id);
        true
    }
    pub fn selection(&self, index: usize) -> Result<Selection, PropError> {
        let inventory = self
            .inventory()
            .ok_or_else(|| PropError::new("No vendor is selected.", ""))?;
        let product = self
            .selected_product()
            .ok_or_else(|| PropError::new("No product is selected.", ""))?;
        let model = product
            .models
            .get(index)
            .ok_or_else(|| PropError::new("The selected model no longer exists.", ""))?;
        Ok(Selection {
            vendor: inventory.vendor.clone(),
            product: product.clone(),
            model: model.clone(),
        })
    }
}
impl Selection {
    pub fn apply_metadata(&self, prop: &mut Prop) {
        if self.model.kind == ModelType::XModel {
            prop.physical.width = self.product.width.clone();
            prop.physical.height = self.product.height.clone();
            prop.physical.depth = self.product.thickness.clone();
            prop.physical.material = self.product.material.clone();
            prop.physical.bulb_type = self.product.pixel_description.clone();
            prop.physical.node_count = self.product.pixel_count.to_string();
            prop.information.notes = self.product.notes.clone();
            prop.prop_type = self.product.product_type.clone();
        }
        let vendor = &mut prop.vendor;
        for (target, value) in [
            (&mut vendor.name, &self.vendor.name),
            (&mut vendor.contact, &self.vendor.contact),
            (&mut vendor.email, &self.vendor.email),
            (&mut vendor.phone, &self.vendor.phone),
        ] {
            if target.is_empty() {
                *target = value.clone();
            }
        }
        if vendor.website.is_empty() {
            if let Some(link) = self.vendor.links.iter().find(|l| l.name == "Website") {
                vendor.website = link.url.clone();
            }
        }
    }
}
