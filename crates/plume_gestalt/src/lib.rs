use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

#[repr(C)]
pub struct __CFString(c_void);
pub type CFStringRef = *const __CFString;
pub type CFTypeRef = *const c_void;

const UTF8: u32 = 0x0800_0100;

// ----------------------------------------------------------
//  MobileGestalt + CoreFoundation bindings
// ----------------------------------------------------------

unsafe extern "C" {
    pub fn MGCopyAnswer(key: CFStringRef) -> CFTypeRef;

    pub fn CFStringGetCStringPtr(string: CFStringRef, encoding: u32) -> *const c_char;

    pub fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;

    pub fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;

    pub fn CFRelease(cf: CFTypeRef);
}

// ----------------------------------------------------------
// Create a CFStringRef from a Rust &str
// ----------------------------------------------------------

unsafe fn cfstring_from_str(s: &str) -> CFStringRef {
    let cstr = std::ffi::CString::new(s).unwrap();
    unsafe { CFStringCreateWithCString(std::ptr::null(), cstr.as_ptr(), UTF8) }
}

// ----------------------------------------------------------
// Convert CFStringRef → Rust String
// ----------------------------------------------------------

unsafe fn cfstring_to_string(cf: CFStringRef) -> Option<String> {
    if cf.is_null() {
        return None;
    }

    unsafe {
        let ptr = CFStringGetCStringPtr(cf, UTF8);
        if !ptr.is_null() {
            return Some(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }

        let mut buf = [0i8; 256];
        if CFStringGetCString(cf, buf.as_mut_ptr(), buf.len() as isize, UTF8) {
            return Some(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned());
        }
    }

    None
}

// ----------------------------------------------------------
// UDID
// ----------------------------------------------------------

pub fn get_udid() -> Option<String> {
    unsafe {
        let key = cfstring_from_str("UniqueDeviceID");
        let cf = MGCopyAnswer(key);

        if cf.is_null() {
            CFRelease(key as CFTypeRef);
            return None;
        }

        let result = cfstring_to_string(cf as CFStringRef);

        CFRelease(cf);
        CFRelease(key as CFTypeRef);

        result
    }
}
