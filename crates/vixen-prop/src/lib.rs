//! Custom props: the document a Custom Prop Editor window edits, its files, and the editor itself with no window.
//!
//! Ported from `Vixen.Modules/App/CustomPropEditor`. One `Prop` belongs to one editor; nothing here is shared
//! between windows, unlike upstream's `PropModelServices` singleton.

pub mod canvas;
pub mod clip;
pub mod editor;
pub mod file;
pub mod legacy;
pub mod model;
pub mod states;
pub mod scale;
pub mod xmodel;
pub mod vendor;

pub use canvas::{AdornerGeometry, AdornerHandle, Axis, Edge, LightDot, Modifiers, SizeChange};
pub use clip::PropClip;
pub use editor::{
    cleanse_name, new_prop_name, parse_path, path_text, CanvasMode, ElementInfo, OrderRow, PanelTab, PropEditor, PropEditorSettings, PropField,
    RowPath, TreeRow,
};
pub use file::{legacy_backup_path, load_prop, load_prop_bytes, manifest_json, read_package, save_prop, write_package, LoadedProp, PropSourceFormat, SaveOutcome};
pub use model::*;
