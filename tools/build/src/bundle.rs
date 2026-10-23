//! Puts the program in a macOS app, SMT4.app beside it, with the MoltenVK it
//! draws through inside, so it runs with nothing from Homebrew and opens from
//! the Finder. Step 8 of the build, with --bundle:
//!
//!     cargo build-game <rom> --bundle
//!
//! The app starts the game it was built from; another ROM dropped on it, or
//! given after `open SMT4.app --args`, goes instead. Like the program, it
//! holds the game's code: don't share it.

use std::path::{Path, PathBuf};

use crate::{fail, run};

const APP: &str = "SMT4.app";
/// where Homebrew puts MoltenVK, Apple Silicon's then Intel's
const MOLTENVK: [&str; 2] = ["/opt/homebrew/opt/molten-vk", "/usr/local/opt/molten-vk"];

/// what the app runs, to hand the program the ROM it was built from when
/// it's given none
const LAUNCHER: &str = r#"#!/bin/sh
# starts the game with the ROM it was built from, unless given another
here="$(cd "$(dirname "$0")" && pwd)"
rom="$(cat "$here/../Resources/rom-path" 2>/dev/null)"
if [ "$#" -eq 0 ] && [ -f "$rom" ]; then
    set -- "$rom"
fi
exec "$here/smt4-game" "$@"
"#;

fn info_plist() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>SMT4</string>
    <key>CFBundleDisplayName</key>
    <string>Shin Megami Tensei IV for PC</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.smt4-recomp</string>
    <key>CFBundleExecutable</key>
    <string>SMT4</string>
    <key>CFBundleIconFile</key>
    <string>SMT4</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.role-playing-games</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSCameraUsageDescription</key>
    <string>The game can take pictures through the 3DS camera.</string>
    <key>NSLocalNetworkUsageDescription</key>
    <string>Local wireless play with other consoles.</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>3DS game</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>3ds</string>
                <string>cci</string>
                <string>cxi</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
"#
    )
}

/// MoltenVK's library and license, from Homebrew
fn moltenvk() -> (PathBuf, Option<PathBuf>) {
    let found = MOLTENVK.iter().map(Path::new).find(|dir| dir.join("lib").join("libMoltenVK.dylib").is_file());
    let Some(dir) = found else { fail("MoltenVK not found, the app takes it from Homebrew: brew install molten-vk") };
    // the file itself, not the link to it Homebrew's folder holds
    let lib = std::fs::canonicalize(dir.join("lib").join("libMoltenVK.dylib")).unwrap_or_else(|e| fail(&format!("could not read MoltenVK: {e}")));
    let license = Some(dir.join("LICENSE")).filter(|license| license.is_file());
    (lib, license)
}

/// the icon, made from the launcher's picture at the sizes macOS asks for
fn make_icon(root: &Path, resources: &Path, scratch: &Path) {
    let png = root.join("launcher").join("assets").join("icon.png");
    let set = scratch.join("SMT4.iconset");
    let _ = std::fs::remove_dir_all(&set);
    std::fs::create_dir_all(&set).unwrap_or_else(|e| fail(&format!("could not make {}: {e}", set.display())));
    for size in [16, 32, 128, 256, 512] {
        for (scale, suffix) in [(1, ""), (2, "@2x")] {
            let out = set.join(format!("icon_{size}x{size}{suffix}.png"));
            let pixels = (size * scale).to_string();
            let args = [
                "-z".as_ref(),
                pixels.as_ref(),
                pixels.as_ref(),
                png.as_os_str(),
                "--out".as_ref(),
                out.as_os_str(),
            ];
            quietly("sips", &args, root);
        }
    }
    let icns = resources.join("SMT4.icns");
    run("iconutil", &["-c".as_ref(), "icns".as_ref(), set.as_os_str(), "-o".as_ref(), icns.as_os_str()], root);
    let _ = std::fs::remove_dir_all(&set);
}

/// runs a program that talks a lot, its output kept to itself unless it fails
fn quietly(program: &str, args: &[&std::ffi::OsStr], cwd: &Path) {
    let out = std::process::Command::new(program).args(args).current_dir(cwd).output();
    match out {
        Ok(out) if out.status.success() => {}
        Ok(out) => fail(&format!("{program} failed: {}", String::from_utf8_lossy(&out.stderr).trim())),
        Err(error) => fail(&format!("could not start {program}: {error}")),
    }
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|e| fail(&format!("could not write {}: {e}", path.display())));
}

fn copy(from: &Path, to: &Path) {
    std::fs::copy(from, to).unwrap_or_else(|e| fail(&format!("could not copy {} to {}: {e}", from.display(), to.display())));
}

/// makes SMT4.app beside the program, anew each time, and gives where it is
pub fn bundle(root: &Path, work: &Path, exe: &Path, rom: &Path) -> PathBuf {
    if !cfg!(target_os = "macos") {
        fail("--bundle makes a macOS app, and works only on macOS");
    }
    let app = exe.with_file_name(APP);
    let _ = std::fs::remove_dir_all(&app);
    let contents = app.join("Contents");
    let (macos, frameworks, resources) = (contents.join("MacOS"), contents.join("Frameworks"), contents.join("Resources"));
    for dir in [&macos, &frameworks, &resources] {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| fail(&format!("could not make {}: {e}", dir.display())));
    }

    write(&contents.join("Info.plist"), &info_plist());
    // not smt4 beside SMT4, which macOS's file names, blind to case, take
    // for the same
    copy(exe, &macos.join("smt4-game"));
    let launcher = macos.join("SMT4");
    write(&launcher, LAUNCHER);
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| fail(&format!("could not make {} runnable: {e}", launcher.display())));
    }
    write(&resources.join("rom-path"), &format!("{}\n", crate::absolute(rom).display()));

    let (moltenvk, license) = moltenvk();
    println!("MoltenVK: {}", moltenvk.display());
    copy(&moltenvk, &frameworks.join("libMoltenVK.dylib"));
    if let Some(license) = license {
        copy(&license, &resources.join("MoltenVK-LICENSE"));
    }
    make_icon(root, &resources, work);

    // signed ad hoc, inside out: Apple Silicon runs nothing unsigned, and a
    // copied library loses the signature it had
    let sign = |path: &Path| run("codesign", &["--force".as_ref(), "--sign".as_ref(), "-".as_ref(), path.as_os_str()], root);
    sign(&frameworks.join("libMoltenVK.dylib"));
    sign(&macos.join("smt4-game"));
    sign(&app);
    app
}
