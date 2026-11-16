//! The version the Administration window and the updates code report. Port of `VersionInfo`.

/// `Assembly.GetName().Version`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VersionInfo {
    pub major: i32,
    pub minor: i32,
    pub build: i32,
    pub revision: i32,
}

/// The app's version: `Cargo.toml`'s `3.13.4` reads as major 3, minor 13, revision 4. `build` is `CFBundleVersion`, which only the
/// app bundle knows.
pub fn current_version(build: i32) -> VersionInfo {
    version_from_package(env!("CARGO_PKG_VERSION"), build)
}

fn version_from_package(text: &str, build: i32) -> VersionInfo {
    let mut parts = text.split('.').map(|p| p.parse().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let revision = parts.next().unwrap_or(0);
    VersionInfo { major, minor, build, revision }
}

impl VersionInfo {
    pub fn is_test_build(&self) -> bool {
        self.major == 0 && self.minor == 0 && self.build == 0
    }

    pub fn is_dev_build(&self) -> bool {
        self.major == 0 && self.build > 0
    }

    pub fn is_release_build(&self) -> bool {
        self.major > 0
    }

    pub fn current_version_type(&self) -> &'static str {
        if self.is_test_build() {
            "Test"
        } else if self.is_dev_build() {
            "Development"
        } else {
            "Release"
        }
    }

    /// The release version on a release build, the build number on a development build, and empty otherwise.
    pub fn release_version(&self) -> String {
        if self.is_release_build() {
            let mut version = format!("{}.{}", self.major, self.minor);
            if self.revision > 0 {
                version.push_str(&format!("u{}", self.revision));
            }
            version
        } else if self.is_dev_build() {
            self.build.to_string()
        } else {
            String::new()
        }
    }

    pub fn version_name(&self) -> String {
        if self.is_test_build() {
            "Test Build".to_string()
        } else if self.is_dev_build() {
            format!("Development Build #{}", self.build)
        } else {
            format!("Release {}", self.release_version())
        }
    }

    pub fn build_name(&self) -> String {
        if self.build > 0 {
            format!("Build #{}", self.build)
        } else {
            "Build #".to_string()
        }
    }

    /// The tag Help > Release Notes looks up. `None` shows that test builds have none.
    pub fn release_notes_tag(&self) -> Option<String> {
        if self.is_dev_build() {
            Some(format!("DevBuild-{}", self.build))
        } else if self.is_release_build() {
            Some(self.release_version())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(major: i32, minor: i32, build: i32, revision: i32) -> VersionInfo {
        VersionInfo { major, minor, build, revision }
    }

    #[test]
    fn test_build() {
        let v = version(0, 0, 0, 7);
        assert!(v.is_test_build() && !v.is_dev_build() && !v.is_release_build());
        assert_eq!(v.current_version_type(), "Test");
        assert_eq!(v.version_name(), "Test Build");
        assert_eq!(v.release_version(), "");
        assert_eq!(v.build_name(), "Build #");
        assert_eq!(v.release_notes_tag(), None);
    }

    #[test]
    fn development_build() {
        let v = version(0, 2, 1501, 0);
        assert!(!v.is_test_build() && v.is_dev_build() && !v.is_release_build());
        assert_eq!(v.current_version_type(), "Development");
        assert_eq!(v.version_name(), "Development Build #1501");
        assert_eq!(v.release_version(), "1501");
        assert_eq!(v.build_name(), "Build #1501");
        assert_eq!(v.release_notes_tag().as_deref(), Some("DevBuild-1501"));
    }

    #[test]
    fn release_builds() {
        let v = version(3, 13, 1400, 0);
        assert!(!v.is_test_build() && !v.is_dev_build() && v.is_release_build());
        assert_eq!(v.current_version_type(), "Release");
        assert_eq!(v.version_name(), "Release 3.13");
        assert_eq!(v.release_version(), "3.13");
        assert_eq!(v.build_name(), "Build #1400");
        assert_eq!(v.release_notes_tag().as_deref(), Some("3.13"));

        let v = version(3, 13, 1400, 4);
        assert_eq!(v.version_name(), "Release 3.13u4");
        assert_eq!(v.release_version(), "3.13u4");
        assert_eq!(v.release_notes_tag().as_deref(), Some("3.13u4"));
    }

    #[test]
    fn neither_test_nor_development_falls_through_to_release_with_no_version() {
        let v = version(0, 5, 0, 0);
        assert!(!v.is_test_build() && !v.is_dev_build() && !v.is_release_build());
        assert_eq!(v.current_version_type(), "Release");
        assert_eq!(v.version_name(), "Release ");
        assert_eq!(v.release_version(), "");
        assert_eq!(v.build_name(), "Build #");
    }

    #[test]
    fn the_package_version_reads_as_this_release() {
        assert_eq!(version_from_package("3.13.4", 1).version_name(), "Release 3.13u4");
        assert_eq!(current_version(1).release_version(), "3.13u4");
        assert_eq!(current_version(1).build_name(), "Build #1");
    }
}
