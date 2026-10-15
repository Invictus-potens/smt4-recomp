//! sdds4, recompiled ahead of time by 3dsrecomp and run by Zakuro. it
//! still reads the game for everything but the code, so it takes the path
//! to a copy of it the way Zakuro does.

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::ptr::addr_of;

extern "C" {
    static recomp_abi: u32;
    static recomp_entry_count: u32;
    static recomp_entries: u8;
    static recomp_module_count: u32;
    static recomp_modules: u8;
}

/// called by overrides/multipliers.c each time a multiplier boosts a gain.
#[no_mangle]
pub extern "C" fn smt4_log_override(
    resource: *const c_char,
    lr: u32,
    total_before: u32,
    delta_in: i32,
    delta_out: i32,
    num: i32,
    den: i32,
) {
    let resource = if resource.is_null() {
        "?"
    } else {
        // SAFETY: all callers pass a string literal
        unsafe { CStr::from_ptr(resource) }.to_str().unwrap_or("?")
    };
    let line = format!(
        "multiplier: {resource} {delta_in} -> {delta_out} (x{num}/{den}), total before {total_before}, from 0x{lr:08X}"
    );
    log::info!("{line}");
    hook_log(&line);
}

/// appends a line to the file named by SMT4_HOOK_LOG, when it is set, without
/// going through the logger, so it works before and regardless of it.
fn hook_log(line: &str) {
    use std::io::Write;
    let Some(path) = std::env::var_os("SMT4_HOOK_LOG") else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

fn hook_log_start() {
    hook_log("--- sdds4 started, multiplier hooks linked ---");
}

fn main() {
    hook_log_start();
    // SAFETY: the tables 3dsrecomp wrote beside the code build.rs links in
    let code = unsafe {
        zakuro::Linked::new(
            0x00040000000E5C00,
            recomp_abi,
            addr_of!(recomp_entries) as *const c_void,
            recomp_entry_count,
            addr_of!(recomp_modules) as *const c_void,
            recomp_module_count,
        )
    };
    zakuro::run(Some(code));
}
