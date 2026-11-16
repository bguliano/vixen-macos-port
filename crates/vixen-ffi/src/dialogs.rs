//! The property, filter and controller setup dialogs. Thin records and calls over `vixen_session::dialogs`, where the logic and its
//! tests live. Everything here hangs off a [`DialogContext`], which says where a dialog reads and writes: the open profile, or
//! Display Setup's working copy.

mod controllers;
mod e131;
mod elements;
mod filters;
mod fixture;
mod layers;
mod monitors;
mod state;
mod testing;

use crate::core::{FfiError, VixenCore};
use std::sync::Arc;
use uuid::Uuid;
use vixen_session::app::AppCore;
use vixen_session::dialogs::world::{with_world, CoreWorld, DialogWorld, NodeKey, WorldProvider};

/// Where the dialogs read and write.
#[derive(uniffi::Object)]
pub struct DialogContext {
    pub(crate) core: Arc<AppCore>,
    pub(crate) provider: Arc<dyn WorldProvider>,
}

impl DialogContext {
    /// A context on any world. Display Setup makes one on its working copy, so its Cancel drops what the dialogs did.
    pub fn with_provider(core: Arc<AppCore>, provider: Arc<dyn WorldProvider>) -> Arc<DialogContext> {
        Arc::new(DialogContext { core, provider })
    }

    pub(crate) fn world<R>(&self, f: impl FnOnce(&mut dyn DialogWorld) -> R) -> R {
        with_world(&*self.provider, f)
    }
}

#[uniffi::export]
impl DialogContext {
    /// The app, for what's shared between windows, such as the curve editor.
    pub fn vixen_core(&self) -> Arc<VixenCore> {
        Arc::new(VixenCore { inner: Arc::clone(&self.core) })
    }
}

#[uniffi::export]
impl VixenCore {
    /// A context on the open profile. What dialogs do through it reaches the running graph and controllers at once, and the disk
    /// with the next save.
    pub fn dialog_context(&self) -> Arc<DialogContext> {
        DialogContext::with_provider(Arc::clone(&self.inner), Arc::new(CoreWorld(Arc::clone(&self.inner))))
    }
}

pub(crate) fn parse_id(text: &str) -> Result<Uuid, FfiError> {
    Uuid::parse_str(text).map_err(|_| FfiError::new(format!("{text} is not an id")))
}

/// Finds nodes by id. One that's gone is left out.
pub(crate) fn find_nodes(world: &dyn DialogWorld, ids: &[String]) -> Vec<NodeKey> {
    ids.iter().filter_map(|id| Uuid::parse_str(id).ok().and_then(|id| world.find_node(&id))).collect()
}

pub(crate) fn no_such(what: &str) -> FfiError {
    FfiError::new(format!("That {what} no longer exists."))
}
