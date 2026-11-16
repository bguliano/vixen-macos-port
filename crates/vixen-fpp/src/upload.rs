//! `FppDirectUploadService`: which routes a device takes, F77.

use crate::{Cancel, FppClient, FppError, UploadOutcome};
use std::path::Path;

/// A task line for the wizard: text and a percentage.
pub type Progress<'a> = &'a mut dyn FnMut(&str, u32);

pub struct DirectUpload<'a> {
    client: &'a FppClient,
    is_esp_pixel_stick: bool,
    zip: bool,
}

impl<'a> DirectUpload<'a> {
    pub fn new(client: &'a FppClient, is_esp_pixel_stick: bool, supports_zip_archives: bool) -> DirectUpload<'a> {
        DirectUpload { client, is_esp_pixel_stick, zip: supports_zip_archives }
    }

    /// Asks the device what it is.
    pub fn detect(client: &'a FppClient, cancel: &Cancel) -> Result<DirectUpload<'a>, FppError> {
        let info = client.system_info(cancel)?.ok_or_else(|| FppError::Response("The device did not provide valid system information.".into()))?;
        let esp = info.platform == "ESPixelStick";
        Ok(DirectUpload::new(client, esp, esp && info.zip))
    }

    pub fn is_esp_pixel_stick(&self) -> bool {
        self.is_esp_pixel_stick
    }

    pub fn supports_zip_archives(&self) -> bool {
        self.is_esp_pixel_stick && self.zip
    }

    /// Audio, the universe file and the fppd restart: a Falcon Player's, not an ESPixelStick's.
    pub fn supports_fpp_extras(&self) -> bool {
        !self.is_esp_pixel_stick
    }

    pub fn upload_sequence_file(&self, source: &Path, filename: &str, progress: Progress, cancel: &Cancel) -> Result<UploadOutcome, FppError> {
        progress(&format!("Uploading {filename}"), 0);
        let outcome = if self.is_esp_pixel_stick {
            self.client.upload_esp_sequence(filename, source, cancel).map(|_| UploadOutcome::default())?
        } else {
            self.client.upload_sequence(filename, source, cancel)?
        };
        progress(&format!("Uploaded {filename}"), 100);
        Ok(outcome)
    }

    pub fn upload_archive_file(&self, source: &Path, filename: &str, cancel: &Cancel) -> Result<(), FppError> {
        if !self.supports_zip_archives() {
            return Err(FppError::Argument("The detected device does not support ZIP archives.".into()));
        }
        self.client.upload_esp_archive(filename, source, cancel)
    }

    pub fn reboot_esp_pixel_stick(&self, cancel: &Cancel) -> Result<(), FppError> {
        self.client.reboot_esp(cancel)
    }

    pub fn upload_audio_file(&self, source: &Path, filename: &str, progress: Progress, cancel: &Cancel) -> Result<UploadOutcome, FppError> {
        progress(&format!("Uploading {filename}"), 0);
        let outcome = self.client.upload_music(filename, source, cancel)?;
        progress(&format!("Uploaded {filename}"), 100);
        Ok(outcome)
    }

    /// Moves the device's `co-universes.json` aside under `backup_name`. A device without one is left alone.
    pub fn backup_universe_file(&self, backup_name: &str, cancel: &Cancel) -> Result<(), FppError> {
        self.client.rename_file("config", "co-universes.json", backup_name, cancel)
    }

    pub fn upload_universe_file(&self, source: &Path, cancel: &Cancel) -> Result<UploadOutcome, FppError> {
        self.client.upload_file("config", "co-universes.json", source, cancel)
    }

    pub fn restart_fppd(&self, quick: bool, cancel: &Cancel) -> Result<(), FppError> {
        self.client.restart_fppd(quick, cancel)
    }
}
