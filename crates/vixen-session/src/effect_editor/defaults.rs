//! Saved effect defaults for the open profile (F8, F9): the store in `vixen_io`, shared by every window. New effects read it through `new_effect_params`.

use crate::app::AppCore;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use vixen_effects::framework::{EffectDescriptor, Params};
use vixen_io::effect_defaults::EffectDefaultsStore;

type Stores = Mutex<Vec<(PathBuf, Arc<Mutex<EffectDefaultsStore>>)>>;

fn stores() -> &'static Stores {
    static STORES: OnceLock<Stores> = OnceLock::new();
    STORES.get_or_init(Stores::default)
}

impl AppCore {
    /// Read on first use, and again if the profile's root changes, as upstream's is.
    pub fn effect_defaults(&self) -> Arc<Mutex<EffectDefaultsStore>> {
        let root = self.paths.root().to_path_buf();
        let mut all = stores().lock();
        if let Some((_, store)) = all.iter().find(|(r, _)| *r == root) {
            return Arc::clone(store);
        }
        let store = Arc::new(Mutex::new(EffectDefaultsStore::load(&self.paths)));
        all.push((root, Arc::clone(&store)));
        store
    }

    /// What a new effect starts with: the saved default of its type, or the built-in values. Draw on the timeline, Add Multiple Effects, add at marks, toolbox drops and replacing with another type use this; clone, paste and LipSync don't, because they overwrite the data.
    pub fn new_effect_params(&self, descriptor: &EffectDescriptor) -> Params {
        self.effect_defaults().lock().create_params(descriptor).unwrap_or_else(|| descriptor.new_params())
    }
}
