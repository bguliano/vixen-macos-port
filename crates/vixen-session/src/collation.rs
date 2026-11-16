//! Culture-aware UI name ordering. Cache one native comparison key per item, so
//! large mapping lists do not allocate strings in their comparison loop.
pub(crate) fn sort_by_name<T>(rows: &mut [T], name: impl Fn(&T) -> &str) {
    rows.sort_by_cached_key(|row| Key::new(name(row)));
}

#[cfg(target_os = "macos")]
struct Key {
    native: *const std::ffi::c_void,
    fallback: String,
}
#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithBytes(
        allocator: *const std::ffi::c_void,
        bytes: *const u8,
        length: isize,
        encoding: u32,
        external: u8,
    ) -> *const std::ffi::c_void;
    fn CFStringCompare(a: *const std::ffi::c_void, b: *const std::ffi::c_void, flags: u64)
        -> isize;
    fn CFRelease(value: *const std::ffi::c_void);
}
#[cfg(target_os = "macos")]
impl Key {
    fn new(s: &str) -> Self {
        Self {
            native: unsafe {
                CFStringCreateWithBytes(
                    std::ptr::null(),
                    s.as_ptr(),
                    s.len() as isize,
                    0x08000100,
                    0,
                )
            },
            fallback: s.to_owned(),
        }
    }
}
#[cfg(target_os = "macos")]
impl Drop for Key {
    fn drop(&mut self) {
        if !self.native.is_null() {
            unsafe { CFRelease(self.native) }
        }
    }
}
#[cfg(target_os = "macos")]
impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.native.is_null() || other.native.is_null() {
            return self.fallback.cmp(&other.fallback);
        }
        // CFString.h: kCFCompareLocalized (32) uses the user's current locale.
        unsafe { CFStringCompare(self.native, other.native, 32).cmp(&0) }
    }
}
#[cfg(target_os = "macos")]
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
#[cfg(target_os = "macos")]
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
#[cfg(target_os = "macos")]
impl Eq for Key {}
#[cfg(not(target_os = "macos"))]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
struct Key(String);
#[cfg(not(target_os = "macos"))]
impl Key {
    fn new(s: &str) -> Self {
        Self(s.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ui_names_sort_by_letters_before_case() {
        let mut rows = vec!["Zebra", "apple", "Banana"];
        super::sort_by_name(&mut rows, |s| s);
        assert_eq!(rows, vec!["apple", "Banana", "Zebra"]);
    }
}
