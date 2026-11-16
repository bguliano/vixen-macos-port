//! `AudioDevice.settings`: the audio preferences, kept beside `Settings.xml`. Port of `AudioPreferences`.

use crate::error::IoError;
use crate::file::write_atomic;
use crate::xml::{write_document, DocStyle};
use std::path::Path;
use vixen_xml::{XElement, XNode};

const FILE_NAME: &str = "AudioDevice.settings";

/// `AudioOutputManager.DefaultDevicePlaceholder`.
const DEFAULT_DEVICE: &str = "-0";
/// `AudioPreferences.Latency`'s standard value.
pub const DEFAULT_LATENCY_MS: i32 = 25;

/// The file's members. The WASAPI ones mean nothing on a Mac, and stay in the file for a Windows Vixen to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioPreferences {
    pub sound_out_device_id: String,
    pub latency_ms: i32,
    root: XElement,
}

impl Default for AudioPreferences {
    /// `SetStandardValues`.
    fn default() -> AudioPreferences {
        let mut root = XElement::new("", "Settings");
        for (name, value) in [("AudioOutputMode", "WASAPI"), ("AudioClientShareMode", "Shared")] {
            root.push(XElement::with_text("", name, value));
        }
        AudioPreferences { sound_out_device_id: DEFAULT_DEVICE.to_string(), latency_ms: DEFAULT_LATENCY_MS, root }
    }
}

/// A missing, blank or unreadable file gives the standard values. `PreferenceBase.Load` would throw on the last.
pub fn load_audio_preferences(dir: &Path) -> AudioPreferences {
    let Ok(root) = XElement::from_file(&dir.join(FILE_NAME)) else {
        return AudioPreferences::default();
    };
    let defaults = AudioPreferences::default();
    AudioPreferences {
        sound_out_device_id: root.child("SoundOutDeviceId").map(|e| e.text()).unwrap_or(defaults.sound_out_device_id),
        latency_ms: root.child_text("Latency").and_then(|t| t.parse().ok()).unwrap_or(defaults.latency_ms),
        root,
    }
}

/// Writes the file. Members this port doesn't use stay as they were.
pub fn save_audio_preferences(preferences: &AudioPreferences, dir: &Path) -> Result<(), IoError> {
    let mut root = preferences.root.clone();
    set_member(&mut root, "SoundOutDeviceId", &preferences.sound_out_device_id);
    set_member(&mut root, "Latency", &preferences.latency_ms.to_string());
    write_atomic(&dir.join(FILE_NAME), write_document(&root, DocStyle::default()).as_bytes())
}

fn set_member(root: &mut XElement, name: &str, value: &str) {
    match root.child_mut(name) {
        Some(member) => {
            member.children.clear();
            member.children.push(XNode::Text(value.to_string()));
        }
        None => {
            root.push(XElement::with_text("", name, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_blank_or_bad_files_give_the_standard_values() {
        let dir = crate::test_dir("audio-defaults");
        assert_eq!(load_audio_preferences(&dir).latency_ms, 25);
        std::fs::write(dir.join(FILE_NAME), "   ").unwrap();
        assert_eq!(load_audio_preferences(&dir).latency_ms, 25);
        std::fs::write(dir.join(FILE_NAME), "<Settings><Latency>").unwrap();
        let prefs = load_audio_preferences(&dir);
        assert_eq!(prefs.latency_ms, 25);
        assert_eq!(prefs.sound_out_device_id, "-0");
    }

    #[test]
    fn reads_what_a_windows_vixen_wrote_and_keeps_its_other_members() {
        let dir = crate::test_dir("audio-windows");
        std::fs::write(
            dir.join(FILE_NAME),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<Settings xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><SoundOutDeviceId>{0.0.0}</SoundOutDeviceId><AudioOutputMode>WASAPI</AudioOutputMode><AudioClientShareMode>Exclusive</AudioClientShareMode><Latency>60</Latency><Future>kept</Future></Settings>",
        )
        .unwrap();
        let mut prefs = load_audio_preferences(&dir);
        assert_eq!(prefs.latency_ms, 60);
        assert_eq!(prefs.sound_out_device_id, "{0.0.0}");
        prefs.latency_ms = 105;
        save_audio_preferences(&prefs, &dir).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(text.contains("<Latency>105</Latency>"), "{text}");
        assert!(text.contains("<AudioClientShareMode>Exclusive</AudioClientShareMode>"), "{text}");
        assert!(text.contains("<Future>kept</Future>"), "{text}");
        assert_eq!(load_audio_preferences(&dir).latency_ms, 105);
    }

    #[test]
    fn saving_the_standard_values_writes_all_the_members() {
        let dir = crate::test_dir("audio-new");
        save_audio_preferences(&AudioPreferences::default(), &dir).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        for member in ["<SoundOutDeviceId>-0<", "<AudioOutputMode>WASAPI<", "<AudioClientShareMode>Shared<", "<Latency>25<"] {
            assert!(text.contains(member), "{member} in {text}");
        }
    }
}
