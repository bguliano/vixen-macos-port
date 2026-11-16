//! The few Objective-C runtime calls the renderer needs that wgpu doesn't make for it.

use std::ffi::{c_char, c_void, CStr};

#[link(name = "objc")]
extern "C" {
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    static kCGColorSpaceSRGB: *const c_void;
    fn CGColorSpaceCreateWithName(name: *const c_void) -> *mut c_void;
    fn CGColorSpaceRelease(space: *mut c_void);
}

// Linked for `objc_getClass("CAMetalLayer")`, which needs the class to be loaded.
#[link(name = "QuartzCore", kind = "framework")]
extern "C" {}

/// Drains what Metal autoreleases during a frame. A thread Rust started has no pool of its own, and a
/// drawable stuck in a pool that never drains is a drawable `nextDrawable` can never hand out again.
pub(crate) struct AutoreleasePool(*mut c_void);

impl AutoreleasePool {
    pub(crate) fn new() -> AutoreleasePool {
        // SAFETY: no preconditions.
        AutoreleasePool(unsafe { objc_autoreleasePoolPush() })
    }
}

impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        // SAFETY: pushed by `new`, and a guard that isn't `Send` pops on the thread that pushed it.
        unsafe { objc_autoreleasePoolPop(self.0) }
    }
}

fn selector(name: &CStr) -> *mut c_void {
    // SAFETY: `name` is a valid C string, and selectors live forever.
    unsafe { sel_registerName(name.as_ptr()) }
}

/// A clear error is better than wgpu's assert, and the likely mistake is passing a view's plain `CALayer`.
///
/// # Safety
/// `object` must be a valid Objective-C object.
pub(crate) unsafe fn is_metal_layer(object: *mut c_void) -> bool {
    // SAFETY: the class name is a valid C string, and the send has the signature of -isKindOfClass:.
    unsafe {
        let class = objc_getClass(c"CAMetalLayer".as_ptr());
        if class.is_null() {
            return false;
        }
        let send: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> bool =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        send(object, selector(c"isKindOfClass:"), class)
    }
}

/// Undoes two things `wgpu::Surface::configure` sets on the layer, every time it is called:
///
/// - `allowsNextDrawableTimeout = NO`, under which `nextDrawable` can block forever. With the timeout, the
///   render thread can always get back to its commands, and shutting down always finishes.
/// - `colorspace = nil`, under which nothing is colour matched and our sRGB values go to a wide-gamut panel
///   as they are, more saturated than the Swift UI around them.
///
/// # Safety
/// `layer` must be a valid `CAMetalLayer`.
pub(crate) unsafe fn apply_layer_settings(layer: *mut c_void) {
    // SAFETY: each send has the signature of the property setter it calls, the colour space is created and
    // released here, and the layer retains its own reference.
    unsafe {
        let set_timeout: extern "C" fn(*mut c_void, *mut c_void, bool) =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        set_timeout(layer, selector(c"setAllowsNextDrawableTimeout:"), true);

        let srgb = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
        if !srgb.is_null() {
            let set_colorspace: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            set_colorspace(layer, selector(c"setColorspace:"), srgb);
            CGColorSpaceRelease(srgb);
        }
    }
}
