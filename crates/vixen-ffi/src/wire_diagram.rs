//! Options shared by Display Setup and Custom Prop Editor wire diagram exports.
use vixen_session::setup::wire_diagram::WireDiagramDialog;

/// The controls of Wire Diagram Export Options (F128).
#[derive(uniffi::Record, Clone)]
pub struct SetupWireOptions {
    pub all_numbers: bool,
    pub interval_text: String,
    pub marker_size: i32,
    pub label_size: i32,
    pub scale_percent: i32,
}

impl From<WireDiagramDialog> for SetupWireOptions {
    fn from(d: WireDiagramDialog) -> SetupWireOptions {
        SetupWireOptions {
            all_numbers: d.all_numbers,
            interval_text: d.interval_text,
            marker_size: d.node_radius,
            label_size: d.label_font_size,
            scale_percent: d.scale_percent,
        }
    }
}

impl From<SetupWireOptions> for WireDiagramDialog {
    fn from(o: SetupWireOptions) -> WireDiagramDialog {
        WireDiagramDialog {
            all_numbers: o.all_numbers,
            interval_text: o.interval_text,
            node_radius: o.marker_size,
            label_font_size: o.label_size,
            scale_percent: o.scale_percent,
        }
    }
}

/// What the dialog opens with. Nothing is remembered between exports.
#[uniffi::export]
pub fn setup_wire_default_options() -> SetupWireOptions {
    WireDiagramDialog::default().into()
}

/// Why OK is disabled, if it is. This is the message to show.
#[uniffi::export]
pub fn setup_wire_options_error(options: SetupWireOptions) -> Option<String> {
    WireDiagramDialog::from(options).error()
}

