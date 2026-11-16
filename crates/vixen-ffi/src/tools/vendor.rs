//! Vendor Browser bulk snapshots and transport injection. Loading never holds the browser state lock.
use super::xmodel::{XModelChoice, XModelImported};
use super::{now_text, settings, CustomPropEditor};
use crate::core::FfiError;
use parking_lot::Mutex;
use std::sync::Arc;
use vixen_prop::{load_prop_bytes, vendor as v, xmodel::XModelFile, PropEditor};

#[derive(uniffi::Record)]
pub struct PropVendorFetch {
    pub bytes: Vec<u8>,
    pub error: Option<String>,
}
#[uniffi::export(callback_interface)]
pub trait PropVendorClient: Send + Sync {
    fn get(&self, url: String) -> PropVendorFetch;
}
struct Transport(Box<dyn PropVendorClient>);
impl v::Transport for Transport {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let response = self.0.get(url.into());
        if let Some(error) = response.error {
            Err(error)
        } else if response.bytes.len() > v::MAX_DOWNLOAD_BYTES {
            Err("The downloaded file is too large.".into())
        } else {
            Ok(response.bytes)
        }
    }
}
#[derive(Clone, uniffi::Record)]
pub struct PropVendorLink {
    pub name: String,
    pub url: String,
}
#[derive(Clone, uniffi::Record)]
pub struct PropVendorModelLink {
    pub name: String,
    pub description: String,
    pub native: bool,
}
#[derive(Clone, uniffi::Record)]
pub struct PropVendorProduct {
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
    pub models: Vec<PropVendorModelLink>,
}
impl From<&v::Product> for PropVendorProduct {
    fn from(p: &v::Product) -> Self {
        Self {
            name: p.name.clone(),
            product_type: p.product_type.clone(),
            url: p.url.clone(),
            image: p.image.clone(),
            material: p.material.clone(),
            width: p.width.clone(),
            height: p.height.clone(),
            thickness: p.thickness.clone(),
            pixel_count: p.pixel_count,
            pixel_description: p.pixel_description.clone(),
            pixel_spacing: p.pixel_spacing.clone(),
            notes: p.notes.clone(),
            models: p
                .models
                .iter()
                .map(|m| PropVendorModelLink {
                    name: m.name.clone(),
                    description: m.description.clone(),
                    native: m.kind == v::ModelType::Prop,
                })
                .collect(),
        }
    }
}
#[derive(uniffi::Record)]
pub struct PropVendorView {
    pub vendors: Vec<String>,
    pub vendor: Option<u32>,
    pub name: String,
    pub contact: String,
    pub phone: String,
    pub email: String,
    pub logo: Option<String>,
    pub links: Vec<PropVendorLink>,
    pub product: Option<PropVendorProduct>,
}
#[derive(uniffi::Record)]
pub struct PropVendorRow {
    pub path: String,
    pub name: String,
    pub product: bool,
    pub child_count: u32,
}
#[derive(uniffi::Record)]
pub struct PropVendorError {
    pub title: String,
    pub message: String,
}
#[derive(uniffi::Object)]
pub struct PropVendorBrowser {
    state: Mutex<v::Browser>,
    status: Mutex<String>,
}
#[uniffi::export]
impl PropVendorBrowser {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(v::Browser::default()),
            status: Mutex::new(String::new()),
        })
    }
    pub fn load(
        &self,
        client: Box<dyn PropVendorClient>,
        vendor_url: Option<String>,
    ) -> Vec<PropVendorError> {
        let mut errors = Vec::new();
        let inventory = v::load(
            &Transport(client),
            vendor_url.as_deref().unwrap_or(v::VENDOR_URL),
            |s| *self.status.lock() = s,
            |e| {
                vixen_session::logging::warn("VixenModules.App.CustomPropEditor", &e.detail);
                errors.push(PropVendorError {
                    title: e.title,
                    message: e.message,
                });
            },
        );
        for i in &inventory {
            for warning in &i.warnings {
                vixen_session::logging::warn("VixenModules.App.CustomPropEditor", warning);
            }
        }
        self.state.lock().set_inventories(inventory);
        errors
    }
    pub fn status(&self) -> String {
        self.status.lock().clone()
    }
    pub fn view(&self) -> PropVendorView {
        let state = self.state.lock();
        let vendor = state
            .inventory()
            .map(|i| i.vendor.clone())
            .unwrap_or_default();
        PropVendorView {
            vendors: state
                .inventories
                .iter()
                .map(|i| i.vendor.name.clone())
                .collect(),
            vendor: state.vendor.map(|i| i as u32),
            name: vendor.name,
            contact: vendor.contact,
            phone: vendor.phone,
            email: vendor.email,
            logo: vendor.logo,
            links: vendor
                .links
                .into_iter()
                .map(|l| PropVendorLink {
                    name: l.name,
                    url: l.url,
                })
                .collect(),
            product: state.selected_product().map(PropVendorProduct::from),
        }
    }
    pub fn select_vendor(&self, index: u32) {
        self.state.lock().select_vendor(index as usize);
    }
    pub fn select_product(&self, path: String) -> bool {
        self.state.lock().select_product(&path)
    }
    pub fn tree_rows(&self, parent: Option<String>) -> Vec<PropVendorRow> {
        self.state
            .lock()
            .rows(parent.as_deref())
            .into_iter()
            .map(|r| PropVendorRow {
                path: r.path,
                name: r.name,
                product: r.product,
                child_count: r.child_count as u32,
            })
            .collect()
    }
    pub fn download(
        &self,
        index: u32,
        client: Box<dyn PropVendorClient>,
    ) -> Result<Arc<PropVendorDownload>, FfiError> {
        let selected = self.state.lock().selection(index as usize)?;
        use v::Transport as _;
        let bytes=Transport(client).get(&selected.model.url).map_err(|e|FfiError::new(format!("Unable to download the model from the vendor.\nEnsure you have an active internet connection.\n{e}")))?;
        let xmodel = if selected.model.kind == v::ModelType::XModel {
            Some(XModelFile::parse(
                &v::decode_text(&bytes).map_err(FfiError::new)?,
            )?)
        } else {
            None
        };
        Ok(Arc::new(PropVendorDownload {
            selected,
            bytes,
            xmodel,
        }))
    }
}
#[derive(uniffi::Object)]
pub struct PropVendorDownload {
    selected: v::Selection,
    bytes: Vec<u8>,
    xmodel: Option<XModelFile>,
}
#[uniffi::export]
impl PropVendorDownload {
    pub fn choices(&self) -> Vec<XModelChoice> {
        self.xmodel
            .as_ref()
            .map(|f| {
                f.candidates()
                    .iter()
                    .map(|c| XModelChoice {
                        index: c.index as u32,
                        display_name: c.display_name.clone(),
                        model_type: c.model_type.clone(),
                        supported: c.supported,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn import_model(&self, index: u32) -> Result<XModelImported, FfiError> {
        let (mut prop, warnings) = if let Some(file) = &self.xmodel {
            let result = file.import(index as usize, &now_text())?;
            (result.prop, result.warnings)
        } else {
            (load_prop_bytes(&self.bytes, &now_text())?.prop, Vec::new())
        };
        self.selected.apply_metadata(&mut prop);
        // Downloads are untitled documents: Save must never silently overwrite a temporary download.
        Ok(XModelImported {
            editor: Arc::new(CustomPropEditor {
                inner: Mutex::new(PropEditor::imported_document(prop, settings())),
            }),
            warnings,
        })
    }
}
