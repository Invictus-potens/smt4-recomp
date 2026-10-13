use std::env::var;

fn main() {
    let dir = var("CARGO_MANIFEST_DIR").unwrap();
    let family = var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    let env = var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    println!("cargo:rerun-if-changed=code/librecomp.a");
    if env == "msvc" {
        // MSVC's linker looks for recomp.lib, the same archive by its name
        let out = var("OUT_DIR").unwrap();
        std::fs::copy(format!("{dir}/code/librecomp.a"), format!("{out}/recomp.lib")).unwrap();
        println!("cargo:rustc-link-search=native={out}");
    } else {
        println!("cargo:rustc-link-search=native={dir}/code");
    }
    println!("cargo:rustc-link-lib=static=recomp");
    if family == "unix" {
        println!("cargo:rustc-link-lib=m");
    }
    // the code runs on the main thread, whose stack is 8 MiB on Linux and
    // 1 MiB on Windows unless it asks for more
    if family == "windows" {
        let stack = 8 << 20;
        match env.as_str() {
            "msvc" => println!("cargo:rustc-link-arg-bins=/STACK:{stack}"),
            _ => println!("cargo:rustc-link-arg-bins=-Wl,--stack,{stack}"),
        }
    }
}
