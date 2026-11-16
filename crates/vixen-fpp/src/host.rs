use std::net::IpAddr;

/// Test harness traffic must never reach the user's real lighting devices.
pub(crate) fn loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

pub(crate) fn ui_test_address(host: &str, port: Option<&str>) -> Result<String, crate::FppError> {
    if !loopback_host(host) { return Err(crate::FppError::Argument("External devices are disabled in UI tests.".into())); }
    let port = port.unwrap_or("80").parse::<u16>().ok().filter(|port| *port > 0).ok_or_else(|| crate::FppError::Argument("Invalid loopback test port.".into()))?;
    let host = if host.parse::<std::net::Ipv6Addr>().is_ok() { format!("[{host}]") } else { host.to_string() };
    Ok(format!("http://{host}:{port}/"))
}

/// F79, `FppHostValidator.IsHostAddressValid`: an IP address, or a DNS name by .NET's `Uri.CheckHostName` rule.
pub fn is_host_address_valid(host: &str) -> bool {
    if host.trim().is_empty() {
        return false;
    }
    if host.parse::<IpAddr>().is_ok() {
        return true;
    }
    if host.contains(['/', '\\', ':']) || host.len() > 255 {
        return false;
    }
    !host.starts_with(['-', '.'])
        && host
            .strip_suffix('.')
            .unwrap_or(host)
            .split('.')
            .all(|label| !label.is_empty() && label.len() <= 63 && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_fixture_override_cannot_target_a_real_device() {
        assert_eq!(ui_test_address("127.0.0.1",Some("12345")).unwrap(),"http://127.0.0.1:12345/");
        assert_eq!(ui_test_address("::1",Some("12345")).unwrap(),"http://[::1]:12345/");
        for host in ["192.168.1.1","fpp.local","example.com","127.0.0.1.example.com","127.0.0.1:12345"] { assert!(ui_test_address(host,Some("12345")).is_err()); }
        assert!(ui_test_address("127.0.0.1",Some("0")).is_err());
    }

    #[test]
    fn the_values_upstreams_tests_pin() {
        for good in ["192.168.1.50", "::1", "fpp", "fpp.local"] {
            assert!(is_host_address_valid(good), "{good}");
        }
        for bad in ["", "   ", "http://192.168.1.50", "\\\\fpp\\share", "fpp:80", "-fpp", ".fpp", "fpp..local", "f pp"] {
            assert!(!is_host_address_valid(bad), "{bad}");
        }
    }
}
