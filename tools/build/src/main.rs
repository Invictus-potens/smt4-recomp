//! Builds smt4-recomp from your own copy of Shin Megami Tensei IV (USA).
//!
//!     cargo build-game <path to your .3ds or .cci>
//!
//! (an alias, in .cargo/config.toml, for `cargo run --release --manifest-path
//! tools/build/Cargo.toml --`). Does every step in order, skipping the ones
//! already done:
//!
//! 1. checks Rust, a C compiler, ar and git are there;
//! 2. checks the ROM is Shin Megami Tensei IV (USA), title 00040000000E5C00;
//! 3. fetches and builds 3dsrecomp, at the version this project was tested with;
//! 4. recompiles the game's code into C with this project's overrides (the slow
//!    step, 20 minutes or more), into code/;
//! 5. patches the generated C (src/patch.rs here);
//! 6. builds the program, target/release/smt4(.exe);
//! 7. puts the cheats file in Zakuro's cheats folder, adding only the cheats
//!    missing from one already there;
//! 8. with --bundle, on macOS, makes target/release/SMT4.app (src/bundle.rs).
//!
//! Nothing it makes is meant to be shared: code/ and the program hold the
//! game's code.

mod bundle;
mod patch;
mod sha256;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

const TITLE_ID: u64 = 0x0004_0000_000E_5C00;
/// 3dsrecomp, from the fork with the macOS changes until fearkov/3dsrecomp
/// has them; SMT4_RECOMP_URL names another
const RECOMP_URL: &str = "https://github.com/Tony0000/3dsrecomp";
/// the 3dsrecomp version the overrides and patches were made and tested with,
/// c3f21f9 and the macOS changes; SMT4_RECOMP_REV names another
const RECOMP_REV: &str = "a5b2eef";
const NAME: &str = "smt4";
const EXE: &str = std::env::consts::EXE_SUFFIX;

const USAGE: &str = "\
Builds smt4-recomp from your own copy of Shin Megami Tensei IV (USA).

usage: cargo build-game <rom> [--force] [--no-cheats] [--low-priority] [--bundle]
       cargo build-game --patch-only

  <rom>           your Shin Megami Tensei IV (USA) .3ds/.cci
  --force         redo the recompilation even when nothing changed
  --no-cheats     don't copy the cheats file to Zakuro's folder
  --low-priority  build at a lower priority, to keep using the PC meanwhile
  --bundle        on macOS, also make target/release/SMT4.app, MoltenVK inside
  --patch-only    only patch the generated C in code/ and put it in librecomp.a

Steps: 1. tools, 2. ROM, 3. 3dsrecomp, 4. recompiling the game's code (20
minutes or more, into code/), 5. patching the generated C, 6. the program,
target/release/smt4(.exe), 7. Zakuro's cheats file, 8. with --bundle, the
macOS app. Done steps are skipped.
Nothing it makes is meant to be shared: code/ and the program hold the game's
code. CC and AR pick the C compiler and archiver; without them, it uses the
first 64-bit gcc on PATH, or else one installed by winget (WinLibs) or MSYS2.
On macOS that is Apple's clang, from \"xcode-select --install\".
SMT4_RECOMP_URL and SMT4_RECOMP_REV build another 3dsrecomp.";

fn step(text: &str) {
    println!("\n== {text}");
}

pub fn fail(text: &str) -> ! {
    eprintln!("\nerror: {text}");
    std::process::exit(1);
}

/// runs a program, stopping the build if it fails
pub fn run<S: AsRef<OsStr>>(program: impl AsRef<OsStr>, args: &[S], cwd: &Path) {
    let program = program.as_ref();
    let shown: Vec<String> = std::iter::once(program)
        .chain(args.iter().map(AsRef::as_ref))
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    println!("> {}", shown.join(" "));
    let name = Path::new(program).file_name().unwrap_or(program).to_string_lossy().into_owned();
    match Command::new(program).args(args).current_dir(cwd).status() {
        Ok(status) if status.success() => {}
        Ok(_) => fail(&format!("{name} failed, see above")),
        Err(error) => fail(&format!("could not start {name}: {error}")),
    }
}

/// a program's output, or "" if it couldn't run
fn output(program: impl AsRef<OsStr>, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default()
}

/// the files in dir whose names pass keep, sorted by name
pub fn sorted_files(dir: &Path, keep: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.file_name().and_then(OsStr::to_str).is_some_and(&keep))
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().to_lowercase());
    files
}

/// where a program is: itself if it's a path, or found on PATH
fn which(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return path.is_file().then(|| path.to_path_buf());
    }
    let exts: Vec<String> = if cfg!(windows) {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into());
        std::iter::once(String::new()).chain(pathext.split(';').map(str::to_owned)).collect()
    } else {
        vec![String::new()]
    };
    let dirs = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
    dirs.iter()
        .flat_map(|dir| exts.iter().map(move |ext| dir.join(format!("{program}{ext}"))))
        .find(|candidate| candidate.is_file())
}

fn set_env(key: &str, value: impl AsRef<OsStr>) {
    // single-threaded: nothing reads the environment meanwhile
    std::env::set_var(key, value);
}

/// the project folder: the one this program was built in, found from where
/// it runs (tools/build/target/release), or the current one
fn project_root() -> PathBuf {
    let is_root = |dir: &Path| dir.join("overrides").is_dir() && dir.join("Cargo.toml").is_file() && dir.join("cheats").is_dir();
    let exe = std::env::current_exe().ok();
    let found = exe.as_deref().and_then(|exe| exe.ancestors().find(|dir| is_root(dir)).map(Path::to_path_buf));
    let cwd = std::env::current_dir().ok().filter(|dir| is_root(dir));
    found.or(cwd).unwrap_or_else(|| fail("run this from the smt4-recomp folder"))
}

/// the gcc bundled with Strawberry Perl: 64-bit, but with a linker too old
/// for current Rust (it rejects Rust's `-exclude-symbols` directives)
fn is_strawberry(gcc: &Path) -> bool {
    gcc.to_string_lossy().to_lowercase().contains("strawberry")
}

const INSTALL_GCC: &str = if cfg!(target_os = "macos") {
    "Install Apple's command line tools, with \"xcode-select --install\""
} else {
    "Install MinGW-w64 GCC, for example with \"winget install BrechtSanders.WinLibs.POSIX.UCRT\""
};

/// the 3dsrecomp to build: where from, and which version
fn recomp_source() -> (String, String) {
    let env = |key: &str, default: &str| std::env::var(key).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| default.to_owned());
    (env("SMT4_RECOMP_URL", RECOMP_URL), env("SMT4_RECOMP_REV", RECOMP_REV))
}

/// whether gcc builds 64-bit x86 code (or anything, off x86-64)
fn builds_64bit(gcc: &Path) -> bool {
    let machine = output(gcc, &["-dumpmachine"]);
    !machine.is_empty() && (machine.starts_with("x86_64") || !cfg!(target_arch = "x86_64"))
}

/// every gcc to try, in order: the ones on PATH, then where winget's WinLibs
/// and MSYS2 install theirs
fn gcc_candidates() -> Vec<PathBuf> {
    let name = if cfg!(windows) { "gcc.exe" } else { "gcc" };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let packages = PathBuf::from(local).join("Microsoft").join("WinGet").join("Packages");
            let mut winlibs: Vec<PathBuf> = std::fs::read_dir(packages)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|e| e.file_name().to_string_lossy().contains("WinLibs"))
                        .map(|e| e.path().join("mingw64").join("bin"))
                        .collect()
                })
                .unwrap_or_default();
            winlibs.sort();
            dirs.extend(winlibs);
        }
        dirs.extend([r"C:\msys64\ucrt64\bin", r"C:\msys64\mingw64\bin", r"C:\msys64\clang64\bin"].map(PathBuf::from));
    }
    let mut seen = Vec::new();
    for gcc in dirs.into_iter().map(|dir| dir.join(name)) {
        if gcc.is_file() && !seen.contains(&gcc) {
            seen.push(gcc);
        }
    }
    seen
}

/// the C compiler and archiver: CC and AR when set; otherwise the first
/// 64-bit gcc found, so an old 32-bit MinGW first on PATH doesn't get in
/// the way, with the ar next to it
fn find_compiler() -> (String, String) {
    let ar_next_to = |gcc: &Path| {
        let ar = gcc.with_file_name(if cfg!(windows) { "ar.exe" } else { "ar" });
        if ar.is_file() { ar.display().to_string() } else { "ar".to_owned() }
    };
    if let Ok(cc) = std::env::var("CC") {
        if which(&cc).is_none() {
            fail(&format!("CC is {cc}, which isn't there"));
        }
        let ar = std::env::var("AR").unwrap_or_else(|_| which(&cc).map(|gcc| ar_next_to(&gcc)).unwrap_or_else(|| "ar".into()));
        if which(&ar).is_none() {
            fail(&format!("AR is {ar}, which isn't there"));
        }
        if which(&cc).is_some_and(|gcc| is_strawberry(&gcc)) {
            println!("warning: CC is Strawberry Perl's gcc, whose linker is usually too old for current Rust");
        }
        return (cc, ar);
    }
    let candidates = gcc_candidates();
    let usable = |gcc: &&PathBuf| !is_strawberry(gcc) && builds_64bit(gcc);
    match candidates.iter().find(usable) {
        Some(gcc) => {
            for skipped in candidates.iter().take_while(|c| *c != gcc) {
                let why = if is_strawberry(skipped) { "Strawberry Perl's, too old for Rust" } else { "not a 64-bit gcc" };
                println!("skipping {} ({why})", skipped.display());
            }
            let ar = std::env::var("AR").unwrap_or_else(|_| ar_next_to(gcc));
            (gcc.display().to_string(), ar)
        }
        None if candidates.is_empty() => fail(&format!("gcc not found. {INSTALL_GCC}, or set CC and AR to one")),
        None => fail(&format!(
            "no usable 64-bit gcc found, only {} (Strawberry Perl's gcc has a linker too old for \
             current Rust). {INSTALL_GCC}, or set CC and AR to a 64-bit one",
            candidates.iter().map(|g| g.display().to_string()).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// hands the compiler to everything run later, with its folder first on
/// PATH so anything that runs a bare gcc gets the same one
fn use_compiler(cc: &str, ar: &str) {
    set_env("CC", cc);
    set_env("AR", ar);
    if let Some(dir) = which(cc).and_then(|gcc| gcc.parent().map(Path::to_path_buf)) {
        let rest = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
        if let Ok(path) = std::env::join_paths(std::iter::once(dir).chain(rest)) {
            set_env("PATH", path);
        }
    }
}

fn check_tools() {
    step("1. tools");
    for (tool, hint) in [("cargo", "install Rust from https://rustup.rs"), ("git", "install git from https://git-scm.com")] {
        if which(tool).is_none() {
            fail(&format!("{tool} not found: {hint}"));
        }
    }
    let (cc, ar) = find_compiler();
    let machine = output(&cc, &["-dumpmachine"]);
    let rustc = output("rustc", &["-vV"]);
    let host = rustc.lines().find_map(|l| l.strip_prefix("host: ")).unwrap_or("").to_owned();
    let or_unknown = |s: &str| if s.is_empty() { "?".to_owned() } else { s.to_owned() };
    println!("C compiler: {} ({})", which(&cc).unwrap().display(), or_unknown(&machine));
    println!("Rust: {}", or_unknown(&host));
    if !machine.is_empty() && !machine.starts_with("x86_64") && cfg!(target_arch = "x86_64") {
        fail(&format!(
            "CC is {cc}, which builds for {machine}, not 64-bit. Set CC and AR to the full paths \
             of a 64-bit MinGW-w64 gcc and ar, or leave them unset to use the first 64-bit gcc found"
        ));
    }
    if cfg!(windows) && host.ends_with("msvc") {
        fail(
            "Rust uses the MSVC toolchain, but the C side is built with MinGW-w64 here. Run \
             \"rustup default stable-x86_64-pc-windows-gnu\" and try again",
        );
    }
    use_compiler(&cc, &ar);
    if cfg!(target_os = "macos") {
        // the C built for the macOS rustc builds for, rather than for the one
        // it runs on, so the objects link with the program without a warning
        // for each
        if std::env::var_os("MACOSX_DEPLOYMENT_TARGET").is_none() {
            set_env("MACOSX_DEPLOYMENT_TARGET", "11.0");
        }
        // the game draws through Vulkan, which macOS has only by MoltenVK
        let molten = ["/opt/homebrew/lib", "/usr/local/lib"]
            .iter()
            .any(|dir| ["libMoltenVK.dylib", "libvulkan.1.dylib"].iter().any(|lib| Path::new(dir).join(lib).is_file()));
        if !molten {
            println!("warning: MoltenVK not found, the game needs it to start: brew install molten-vk");
        }
    }
    // run by `cargo run`, this program has this package's variables, which
    // aren't the game's or 3dsrecomp's
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy();
        if ["CARGO_PKG_", "CARGO_MANIFEST_", "CARGO_BIN_", "CARGO_CRATE_", "CARGO_PRIMARY_"].iter().any(|p| key.starts_with(p)) {
            std::env::remove_var(&*key);
        }
    }
    // 3dsrecomp's rust-toolchain.toml asks for "stable", which on Windows
    // means the MSVC one even when the GNU one is the default: build it with
    // the same toolchain as this project
    if which("rustup").is_some() {
        let active = output("rustup", &["show", "active-toolchain"]);
        if let Some(toolchain) = active.split(' ').next().filter(|t| !t.is_empty()) {
            set_env("RUSTUP_TOOLCHAIN", toolchain);
            println!("Rust toolchain: {toolchain}");
        }
    }
}

/// the title of a decrypted .3ds/.cci (NCSD) or .cxi (NCCH), from its header
fn rom_title(header: &[u8]) -> Option<u64> {
    let read = |at: usize| u64::from_le_bytes(header[at..at + 8].try_into().unwrap());
    match header.get(0x100..0x104)? {
        b"NCSD" => Some(read(0x108)),
        b"NCCH" => Some(read(0x118)),
        _ => None,
    }
}

fn check_rom(rom: &Path) {
    step("2. ROM");
    let name = rom.file_name().unwrap_or(rom.as_os_str()).to_string_lossy();
    if !rom.is_file() {
        fail(&format!("{} not found", rom.display()));
    }
    let mut header = [0u8; 0x200];
    let read = std::fs::File::open(rom).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut header));
    let title = read.ok().and_then(|_| rom_title(&header));
    let Some(title) = title else {
        fail(&format!("{name} is not a decrypted .3ds/.cci/.cxi (no NCSD or NCCH header)"))
    };
    if title != TITLE_ID {
        fail(&format!(
            "{name} is title {title:016X}, this project is for Shin Megami Tensei IV (USA), {TITLE_ID:016X}"
        ));
    }
    let size = rom.metadata().map(|m| m.len()).unwrap_or(0);
    println!("{name}: Shin Megami Tensei IV (USA), {:.2} GiB", size as f64 / (1u64 << 30) as f64);
}

fn build_3dsrecomp(root: &Path, work: &Path) -> PathBuf {
    let (url, rev) = recomp_source();
    step(&format!("3. 3dsrecomp ({rev})"));
    let src = work.join("3dsrecomp");
    if !src.join(".git").is_dir() {
        let _ = std::fs::create_dir_all(work);
        run("git", &[OsStr::new("clone"), OsStr::new("--quiet"), OsStr::new(&url), src.as_os_str()], root);
    }
    let src_str = src.to_string_lossy();
    // a clone from before 3dsrecomp came from elsewhere
    if output("git", &["-C", &src_str, "remote", "get-url", "origin"]) != url {
        run("git", &["remote", "set-url", "origin", url.as_str()], &src);
    }
    let commit = format!("{rev}^{{commit}}");
    if output("git", &["-C", &src_str, "rev-parse", "--verify", "--quiet", &commit]).is_empty() {
        run("git", &["fetch", "--quiet", "origin"], &src);
    }
    run("git", &["checkout", "--quiet", rev.as_str()], &src);
    run("cargo", &["build", "--release", "--quiet"], &src);
    src.join("target").join("release").join(format!("3dsrecomp{EXE}"))
}

/// what step 4's output depends on, the system and the compiler too, so a
/// code/ built on another doesn't pass for this one's
fn inputs_stamp(root: &Path, rom: &Path) -> String {
    let mut h = sha256::Sha256::new();
    h.update(recomp_source().1.as_bytes());
    let rustc = output("rustc", &["-vV"]);
    h.update(rustc.lines().find_map(|l| l.strip_prefix("host: ")).unwrap_or("").as_bytes());
    h.update(output(std::env::var("CC").unwrap_or_default(), &["-dumpmachine"]).as_bytes());
    h.update(rom.metadata().map(|m| m.len()).unwrap_or(0).to_string().as_bytes());
    for path in sorted_files(&root.join("overrides"), |name| name.ends_with(".c") || name.ends_with(".h")) {
        h.update(path.file_name().unwrap().to_string_lossy().as_bytes());
        h.update(&std::fs::read(&path).unwrap_or_else(|e| fail(&format!("could not read {}: {e}", path.display()))));
    }
    h.hex()
}

pub fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

fn port(root: &Path, work: &Path, recomp: &Path, rom: &Path, force: bool) {
    step("4. recompiling the game's code");
    let code = root.join("code");
    let stamp_file = code.join(".smt4-stamp");
    let stamp = inputs_stamp(root, rom);
    let up_to_date = code.join("librecomp.a").is_file()
        && std::fs::read_to_string(&stamp_file).is_ok_and(|text| text.trim() == stamp);
    if !force && up_to_date {
        println!("code/ is up to date, skipping (--force redoes it)");
        return;
    }
    println!("This takes 20 minutes or more, and a few GB of memory.");
    let out = work.join("port");
    let _ = std::fs::remove_dir_all(&out);
    let args = [
        OsStr::new("port"),
        absolute(rom).as_os_str(),
        absolute(&out).as_os_str(),
        OsStr::new("--name"),
        OsStr::new(NAME),
        OsStr::new("--overrides"),
        absolute(&root.join("overrides")).as_os_str(),
    ]
    .map(OsStr::to_os_string);
    run(recomp, &args, root);
    let made = out.join("code").join("librecomp.a");
    if !made.is_file() {
        fail(&format!("3dsrecomp port did not write {}", made.display()));
    }
    // only code/ is kept: port also writes a Cargo project of its own, and
    // this one's src/main.rs defines things the overrides use
    let _ = std::fs::remove_dir_all(&code);
    std::fs::rename(out.join("code"), &code).unwrap_or_else(|e| fail(&format!("could not move the code to {}: {e}", code.display())));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::write(&stamp_file, format!("{stamp}\n")).unwrap_or_else(|e| fail(&format!("could not write {}: {e}", stamp_file.display())));
}

fn build(root: &Path) -> PathBuf {
    step("6. building the program");
    run("cargo", &["build", "--release"], root);
    let exe = root.join("target").join("release").join(format!("{NAME}{EXE}"));
    if !exe.is_file() {
        fail(&format!("{} was not built", exe.display()));
    }
    exe
}

fn zakuro_dir() -> PathBuf {
    let env = |key: &str| std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from);
    let home = || env("HOME").unwrap_or_else(|| PathBuf::from("."));
    if cfg!(windows) {
        env("APPDATA").unwrap_or_else(|| fail("APPDATA is not set")).join("zakuro")
    } else if cfg!(target_os = "macos") {
        home().join("Library").join("Application Support").join("zakuro")
    } else {
        env("XDG_DATA_HOME").unwrap_or_else(|| home().join(".local").join("share")).join("zakuro")
    }
}

/// the cheats in a cheats file, by name in the order they come, each with
/// its lines
fn cheat_blocks(text: &str) -> Vec<(String, Vec<String>)> {
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    let mut current: Option<usize> = None;
    for line in text.lines() {
        if line.starts_with('[') && line.trim_end().ends_with(']') {
            let name = line.trim().to_owned();
            // a name seen twice keeps its place and the last lines
            current = Some(match blocks.iter().position(|(n, _)| *n == name) {
                Some(i) => {
                    blocks[i].1 = vec![line.to_owned()];
                    i
                }
                None => {
                    blocks.push((name, vec![line.to_owned()]));
                    blocks.len() - 1
                }
            });
        } else if let Some(i) = current {
            blocks[i].1.push(line.to_owned());
        }
    }
    blocks
}

/// the cheats file with the cheats it lacks from ours added at the end, and
/// how many; Zakuro keeps which cheats are on in this same file, so the ones
/// already there are left as they are
fn merge_cheats(theirs: &str, ours: &str) -> (String, usize) {
    let have = cheat_blocks(theirs);
    let missing: Vec<Vec<String>> = cheat_blocks(ours)
        .into_iter()
        .filter(|(name, _)| !have.iter().any(|(n, _)| n == name))
        .map(|(_, lines)| lines)
        .collect();
    let newline = if theirs.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = theirs.lines().map(str::to_owned).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    for block in &missing {
        let end = block.iter().rposition(|l| !l.trim().is_empty()).map_or(0, |i| i + 1);
        lines.push(String::new());
        lines.extend_from_slice(&block[..end]);
    }
    (lines.join(newline) + newline, missing.len())
}

fn install_cheats(root: &Path) {
    step("7. cheats");
    let name = format!("{TITLE_ID:016X}.txt");
    let source = root.join("cheats").join(&name);
    let target = zakuro_dir().join("cheats").join(&name);
    let read = |path: &Path| std::fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("could not read {}: {e}", path.display())));
    let ours = read(&source);
    let count = cheat_blocks(&ours).len();
    let _ = std::fs::create_dir_all(target.parent().unwrap());
    if !target.is_file() {
        std::fs::copy(&source, &target).unwrap_or_else(|e| fail(&format!("could not copy to {}: {e}", target.display())));
        println!("copied {count} cheats to {}", target.display());
        return;
    }
    let text = read(&target);
    let (merged, added) = merge_cheats(&text, &ours);
    if added == 0 {
        println!("{} already has all {count} cheats", target.display());
        return;
    }
    std::fs::write(&target, merged).unwrap_or_else(|e| fail(&format!("could not write {}: {e}", target.display())));
    let kept = cheat_blocks(&text).len();
    println!("added {added} cheats to {}, leaving the {kept} already there as they were", target.display());
}

/// the compilers started from here inherit it
fn lower_priority() {
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        extern "system" {
            fn GetCurrentProcess() -> *mut c_void;
            fn SetPriorityClass(process: *mut c_void, class: u32) -> i32;
        }
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x4000;
        // SAFETY: the pseudo-handle of this process, always valid
        if unsafe { SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS) } == 0 {
            println!("could not lower the priority ({})", std::io::Error::last_os_error());
        }
    }
    #[cfg(unix)]
    {
        extern "C" {
            fn nice(inc: std::ffi::c_int) -> std::ffi::c_int;
        }
        // SAFETY: only changes this process's niceness
        unsafe { nice(10) };
    }
}

struct Args {
    rom: Option<PathBuf>,
    force: bool,
    no_cheats: bool,
    low_priority: bool,
    patch_only: bool,
    bundle: bool,
}

fn parse_args() -> Args {
    let mut args = Args { rom: None, force: false, no_cheats: false, low_priority: false, patch_only: false, bundle: false };
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("-h" | "--help") => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            Some("--force") => args.force = true,
            Some("--no-cheats") => args.no_cheats = true,
            Some("--low-priority") => args.low_priority = true,
            Some("--patch-only") => args.patch_only = true,
            Some("--bundle") => args.bundle = true,
            Some(flag) if flag.starts_with("--") => fail(&format!("unknown option {flag}\n\n{USAGE}")),
            _ if args.rom.is_none() => args.rom = Some(PathBuf::from(arg)),
            _ => fail(&format!("only one ROM, please\n\n{USAGE}")),
        }
    }
    args
}

fn main() {
    let args = parse_args();
    let root = project_root();
    let work = root.join(".build");
    if args.low_priority {
        lower_priority();
    }
    if args.patch_only {
        let (cc, ar) = find_compiler();
        use_compiler(&cc, &ar);
        patch::patch(&root, &cc, &ar);
        return;
    }
    let Some(rom) = args.rom else { fail(&format!("which ROM?\n\n{USAGE}")) };
    if args.bundle && !cfg!(target_os = "macos") {
        fail("--bundle makes a macOS app, and works only on macOS");
    }

    check_tools();
    check_rom(&rom);
    let recomp = build_3dsrecomp(&root, &work);
    port(&root, &work, &recomp, &rom, args.force);
    step("5. patching the generated C");
    patch::patch(&root, &std::env::var("CC").unwrap(), &std::env::var("AR").unwrap());
    let exe = build(&root);
    if !args.no_cheats {
        install_cheats(&root);
    }
    let app = args.bundle.then(|| {
        step("8. the macOS app");
        bundle::bundle(&root, &work, &exe, &rom)
    });
    step("done");
    println!("Run it with:\n  \"{}\" \"{}\"", exe.display(), absolute(&rom).display());
    if let Some(app) = app {
        println!("or open {}, from the Finder or with: open \"{}\"", app.display(), app.display());
    }
    println!(
        "Recommended: Settings, Layout \"Top screen only\", Top screen shape 16:9, and the \
         \"Widescreen 16:9\" cheat. See README.md."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_strawberry_perls_gcc() {
        assert!(is_strawberry(Path::new(r"C:\Strawberry\c\bin\gcc.exe")));
        assert!(!is_strawberry(Path::new(r"C:\msys64\ucrt64\bin\gcc.exe")));
    }

    #[test]
    fn reads_rom_titles() {
        let mut header = [0u8; 0x200];
        assert_eq!(rom_title(&header), None);
        header[0x100..0x104].copy_from_slice(b"NCSD");
        header[0x108..0x110].copy_from_slice(&TITLE_ID.to_le_bytes());
        assert_eq!(rom_title(&header), Some(TITLE_ID));
        header[0x100..0x104].copy_from_slice(b"NCCH");
        header[0x118..0x120].copy_from_slice(&7u64.to_le_bytes());
        assert_eq!(rom_title(&header), Some(7));
    }

    #[test]
    fn adds_only_missing_cheats() {
        let ours = "[Widescreen 16:9]\nD3000000 00000000\n\n[EXP x2]\n00000000 00000002\n\n";
        let theirs = "[Widescreen 16:9]\n*citra_enabled\nD3000000 00000000\r\n\r\n\r\n";
        let theirs = theirs.replace("\r\n", "\n").replace('\n', "\r\n");
        let (merged, added) = merge_cheats(&theirs, ours);
        assert_eq!(added, 1);
        assert_eq!(
            merged,
            "[Widescreen 16:9]\r\n*citra_enabled\r\nD3000000 00000000\r\n\r\n[EXP x2]\r\n00000000 00000002\r\n"
        );
        assert_eq!(merge_cheats(&merged, ours).1, 0);
    }
}
