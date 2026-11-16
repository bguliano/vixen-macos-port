//! Falcon Player and ESPixelStick over HTTP(S): `docs/spec/shows-scheduler-export.md` F76 to F79.
//! Every call blocks, so callers run it off the main thread and stop it through a [`Cancel`].

mod client;
mod error;
mod host;
mod http;
mod models;
mod ping;
mod upload;
mod zip_batch;

pub use client::{escape_data_string, FppClient, Options, UploadOutcome};
pub use error::FppError;
pub use host::is_host_address_valid;
pub use http::Cancel;
pub use ping::ping;
pub use models::{Playlist, PlaylistEntry, PlaylistInfo, SystemInfo, Utilization};
pub use upload::{DirectUpload, Progress};
pub use zip_batch::{phase_count, request_reboot, upload_batch, upload_batches, validate_entries, ArchiveEntry, BatchEvent, BATCH_SIZE};
