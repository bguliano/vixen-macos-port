//! F26 and F28: the two questions the wizard asks a direct upload's device before the run. Both block, so they're asked off the
//! main thread, and both answer through [`ExportWizard`], which drops an answer for an address that has since changed.

use super::{host_unreachable_message, DeviceInfo, ExportWizard, HOST_UNREACHABLE_TITLE};
use crate::logging;
use parking_lot::Mutex;
use std::time::Duration;
use vixen_fpp::{Cancel, FppClient};

/// `PingHost`'s wait.
const PING_TIMEOUT: Duration = Duration::from_millis(500);

/// The format stage's reachability check, for the host box as it is now. Returns the alert's title and message when the host
/// didn't answer and its address is still the one in the box; opening the stage checks without the alert, as upstream.
pub fn check_host(wizard: &Mutex<ExportWizard>, alert: bool) -> Option<(String, String)> {
    let host = wizard.lock().host_to_check()?;
    let reachable = vixen_fpp::ping(&host, PING_TIMEOUT);
    let taken = wizard.lock().set_host_reachable(&host, reachable);
    (alert && taken && !reachable).then(|| (HOST_UNREACHABLE_TITLE.to_string(), host_unreachable_message(&host)))
}

/// `PopulateFppInfoAsync`: what the device says of itself, for the Summary. Returns whether the Summary changed.
pub fn ask_device(wizard: &Mutex<ExportWizard>) -> bool {
    let Some(host) = wizard.lock().device_host() else { return false };
    let info = system_info(&host);
    wizard.lock().set_device_info(&host, info)
}

/// One question to `http://{host}/api/system/info`.
pub fn system_info(host: &str) -> DeviceInfo {
    let answer = FppClient::for_host(host).and_then(|client| client.system_info(&Cancel::new()));
    match answer {
        Ok(Some(info)) => DeviceInfo::Known {
            host_name: info.host_name,
            description: info.host_description,
            platform: info.platform,
            variant: info.variant,
            zip: info.zip,
        },
        // KEEP: an answer of `null` fails upstream too, on the first property read.
        Ok(None) => {
            logging::warn("VixenModules.App.ExportWizard.BulkExportSummaryStage", &format!("Could not retrieve FPP system info for host '{host}'"));
            DeviceInfo::Unavailable
        }
        Err(error) => {
            logging::warn("VixenModules.App.ExportWizard.BulkExportSummaryStage", &format!("Could not retrieve FPP system info for host '{host}': {error}"));
            DeviceInfo::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export_wizard::Stage;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::Path;
    use vixen_io::system::SystemData;
    use vixen_io::DataPaths;

    /// A stand-in device on 127.0.0.1 that gives every request the same answer.
    fn device(status: u16, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let host = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut data = Vec::new();
                let mut chunk = [0u8; 4096];
                while !data.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut chunk) {
                        Ok(n) if n > 0 => data.extend_from_slice(&chunk[..n]),
                        _ => break,
                    }
                }
                let _ = write!(stream, "HTTP/1.1 {status} Whatever\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        host
    }

    #[test]
    fn the_device_describes_itself_or_is_unavailable() {
        let host = device(200, r#"{"HostName":"esps-1","HostDescription":"Roof","Platform":"ESPixelStick","Variant":"ESP32","Zip":true}"#);
        let info = system_info(&host);
        assert_eq!(
            info,
            DeviceInfo::Known {
                host_name: "esps-1".to_string(),
                description: "Roof".to_string(),
                platform: "ESPixelStick".to_string(),
                variant: "ESP32".to_string(),
                zip: true
            }
        );
        assert_eq!(info.esp_pixel_stick_zip(), Some(true));
        assert_eq!(system_info(&device(200, "null")), DeviceInfo::Unavailable);
        assert_eq!(system_info(&device(500, "{}")), DeviceInfo::Unavailable);
        let closed = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        assert_eq!(system_info(&format!("127.0.0.1:{closed}")), DeviceInfo::Unavailable);
        assert_eq!(system_info(""), DeviceInfo::Unavailable);
    }

    #[test]
    fn this_machine_answers_the_reachability_check() {
        let root = std::env::temp_dir().join(format!("vixen-wizard-device-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = DataPaths::new(&root);
        paths.ensure_directories().unwrap();
        std::fs::write(root.join("Sequence/Song.tim"), "").unwrap();
        let system = SystemData::default();
        let exists = |p: &str| Path::new(p).is_dir();
        let mut wizard = ExportWizard::new(paths, Vec::new(), &system);
        assert!(wizard.next(&system, &exists));
        wizard.add_sequences(&[root.join("Sequence/Song.tim")]);
        assert!(wizard.next(&system, &exists));
        let wizard = Mutex::new(wizard);
        assert_eq!(check_host(&wizard, true), None, "not on the format stage, so nothing is asked");
        assert!(!ask_device(&wizard));

        wizard.lock().set_stage_for_test(Stage::OutputFormat);
        wizard.lock().set_direct_upload(true);
        wizard.lock().set_host_text("127.0.0.1");
        assert_eq!(check_host(&wizard, true), None);
        assert!(wizard.lock().host_reachable());

        // Nothing on a Mac answers at the loopback network's other addresses, and nothing sent there leaves the machine.
        if cfg!(target_os = "macos") {
            wizard.lock().set_host_text("127.0.0.2");
            assert_eq!(check_host(&wizard, false), None, "opening the stage asks without the alert");
            assert!(!wizard.lock().host_reachable());
            let alert = check_host(&wizard, true).unwrap();
            assert_eq!(alert.0, "FPP Host Unreachable");
            assert_eq!(alert.1, "FPP host '127.0.0.2' could not be reached. Verify the host is online and try again.");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
