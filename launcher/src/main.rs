//! Launcher for smt4-recomp. It finds your copy of Shin Megami Tensei IV
//! (USA), checks the tools the build needs, gets this project's source,
//! builds the game into a program with tools/build, and starts it.
//!
//! It holds no game code, so unlike the program it builds it can be shared.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use eframe::egui;
use serde::{Deserialize, Serialize};

const TITLE_ID: u64 = 0x0004_0000_000E_5C00;
const REPO: &str = "https://github.com/Invictus-potens/smt4-recomp";
const EXE: &str = if cfg!(windows) { "smt4.exe" } else { "smt4" };

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Shin Megami Tensei IV for PC")
            .with_inner_size([620.0, 640.0])
            .with_min_inner_size([480.0, 420.0])
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native("smt4-launcher", options, Box::new(|cc| Ok(Box::new(Launcher::new(&cc.egui_ctx)))))
}

// ---------------------------------------------------------------------------
// settings

#[derive(Serialize, Deserialize)]
struct Config {
    rom: Option<PathBuf>,
    source: Option<PathBuf>,
    /// look for project updates when the launcher opens
    #[serde(default = "yes")]
    check_updates: bool,
}

fn yes() -> bool {
    true
}

impl Default for Config {
    fn default() -> Config {
        Config { rom: None, source: None, check_updates: true }
    }
}

fn data_dir() -> PathBuf {
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library").join("Application Support"))
    } else {
        std::env::var_os("APPDATA").map(PathBuf::from).or_else(|| home().map(|home| home.join(".config")))
    };
    base.unwrap_or_else(|| PathBuf::from(".")).join("smt4-recomp")
}

impl Config {
    fn load() -> Config {
        std::fs::read_to_string(data_dir().join("launcher.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        let dir = data_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(dir.join("launcher.json"), text);
        }
    }
}

// ---------------------------------------------------------------------------
// the ROM

/// whether path is a decrypted Shin Megami Tensei IV (USA), or why not
fn check_rom(path: &Path) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut header = [0u8; 0x200];
    file.read_exact(&mut header).map_err(|_| "too small to be a 3DS game".to_owned())?;
    let title = match &header[0x100..0x104] {
        b"NCSD" => u64::from_le_bytes(header[0x108..0x110].try_into().unwrap()),
        b"NCCH" => u64::from_le_bytes(header[0x118..0x120].try_into().unwrap()),
        _ => return Err("not a decrypted .3ds/.cci (encrypted, or another kind of file)".into()),
    };
    if title != TITLE_ID {
        return Err(format!("another game (title {title:016X}); this needs Shin Megami Tensei IV (USA)"));
    }
    Ok(())
}

/// the first Shin Megami Tensei IV in folder or the folders right under it
fn find_rom(folder: &Path) -> Option<PathBuf> {
    let mut dirs = vec![folder.to_path_buf()];
    if let Ok(entries) = std::fs::read_dir(folder) {
        dirs.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for path in entries.flatten().map(|e| e.path()) {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if matches!(ext.as_str(), "3ds" | "cci" | "cxi") && check_rom(&path).is_ok() {
                return Some(path);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// tools

#[derive(Clone)]
struct Tool {
    name: &'static str,
    found: Option<String>,
    /// what to do when it's missing or wrong
    hint: String,
    /// a winget package that installs it
    winget: Option<&'static str>,
    /// on macOS, a command that installs it, run as a job
    mac_install: Option<&'static [&'static str]>,
    /// whether the build needs it, rather than only the game
    for_build: bool,
}

#[derive(Clone, Default)]
struct Tools {
    list: Vec<Tool>,
    gcc: Option<PathBuf>,
    rust_msvc: bool,
}

impl Tools {
    fn ok(&self) -> bool {
        !self.list.is_empty() && self.list.iter().filter(|tool| tool.for_build).all(|tool| tool.found.is_some())
    }
}

fn quiet(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

fn output(program: impl AsRef<std::ffi::OsStr>, args: &[&str]) -> Option<String> {
    let out = quiet(Command::new(program).args(args).stdin(Stdio::null())).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let text = if text.is_empty() { String::from_utf8_lossy(&out.stderr).trim().to_owned() } else { text };
    (out.status.success() && !text.is_empty()).then_some(text)
}

fn cargo_bin() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".cargo").join("bin"))
}

/// where Homebrew puts what it installs, which a launcher opened from the
/// Finder doesn't have on PATH
const HOMEBREW_BINS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];

/// a 64-bit gcc: on PATH, or where winget's WinLibs and MSYS2 put theirs,
/// skipping old 32-bit MinGW ones that may come first on PATH. on macOS,
/// Apple's clang answers to gcc
fn find_gcc() -> GccSearch {
    let name = if cfg!(windows) { "gcc.exe" } else { "gcc" };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let packages = PathBuf::from(local).join("Microsoft").join("WinGet").join("Packages");
        if let Ok(entries) = std::fs::read_dir(packages) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().contains("WinLibs") {
                    dirs.push(entry.path().join("mingw64").join("bin"));
                }
            }
        }
    }
    dirs.extend([r"C:\msys64\ucrt64\bin", r"C:\msys64\mingw64\bin"].map(PathBuf::from));
    // anything builds for this machine off x86-64, gcc on Apple Silicon is
    // arm64-apple-darwin
    let is_64 = |gcc: &PathBuf| {
        output(gcc, &["-dumpmachine"]).is_some_and(|machine| machine.starts_with("x86_64") || !cfg!(target_arch = "x86_64"))
    };
    let all: Vec<PathBuf> = dirs.into_iter().map(|dir| dir.join(name)).filter(|gcc| gcc.is_file()).collect();
    match all.iter().find(|gcc| !is_strawberry(gcc) && is_64(gcc)) {
        Some(gcc) => GccSearch::Found(gcc.clone()),
        None => match all.into_iter().find(|gcc| is_strawberry(gcc)) {
            Some(gcc) => GccSearch::OnlyStrawberry(gcc),
            None => GccSearch::None,
        },
    }
}

enum GccSearch {
    Found(PathBuf),
    /// only Strawberry Perl's, whose linker is too old for current Rust
    OnlyStrawberry(PathBuf),
    None,
}

fn is_strawberry(gcc: &Path) -> bool {
    gcc.to_string_lossy().to_lowercase().contains("strawberry")
}

/// Apple's command line tools, clang, ar and git, which macOS offers to
/// install in a window of its own
const XCODE_SELECT: &[&str] = &["xcode-select", "--install"];

fn check_tools() -> Tools {
    let mac = cfg!(target_os = "macos");
    let mut tools = Tools::default();
    let git = output("git", &["--version"]);
    tools.list.push(Tool {
        name: "Git",
        found: git,
        hint: if mac { "Downloads this project's source (Apple's command line tools)." } else { "Downloads this project's source." }.into(),
        winget: Some("Git.Git"),
        mac_install: Some(XCODE_SELECT),
        for_build: true,
    });

    let cargo = cargo_bin().map(|bin| bin.join(if cfg!(windows) { "rustc.exe" } else { "rustc" }));
    let rustc = output("rustc", &["-vV"]).or_else(|| cargo.and_then(|c| output(c, &["-vV"])));
    let host = rustc.as_deref().and_then(|v| v.lines().find_map(|l| l.strip_prefix("host: ")).map(str::to_owned));
    tools.rust_msvc = cfg!(windows) && host.as_deref().is_some_and(|h| h.ends_with("msvc"));
    tools.list.push(Tool {
        name: "Rust",
        found: host.clone().filter(|_| !tools.rust_msvc),
        hint: if tools.rust_msvc {
            "Installed, but set to the MSVC toolchain; the build needs the GNU one.".into()
        } else {
            "Builds the program.".into()
        },
        winget: Some("Rustlang.Rustup"),
        mac_install: Some(&["sh", "-c", "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"]),
        for_build: true,
    });

    let search = find_gcc();
    let hint = match &search {
        GccSearch::OnlyStrawberry(gcc) => format!(
            "Only {} found, from Strawberry Perl: its linker is too old for Rust. Install WinLibs.",
            gcc.display()
        ),
        _ if mac => "Compiles the recompiled game code (Apple's command line tools).".into(),
        _ => "Compiles the recompiled game code (MinGW-w64).".into(),
    };
    tools.gcc = match search {
        GccSearch::Found(gcc) => Some(gcc),
        _ => None,
    };
    tools.list.push(Tool {
        name: if mac { "C compiler" } else { "64-bit GCC" },
        found: tools.gcc.as_ref().map(|p| p.display().to_string()),
        hint,
        winget: Some("BrechtSanders.WinLibs.POSIX.UCRT"),
        mac_install: Some(XCODE_SELECT),
        for_build: true,
    });

    // the game draws through Vulkan, which macOS has only by MoltenVK
    if mac {
        let lib = HOMEBREW_BINS
            .iter()
            .map(|bin| Path::new(bin).with_file_name("lib").join("libMoltenVK.dylib"))
            .find(|lib| lib.is_file());
        tools.list.push(Tool {
            name: "MoltenVK",
            found: lib.map(|lib| lib.display().to_string()),
            hint: "Vulkan for the game to draw with, from Homebrew (brew.sh).".into(),
            winget: None,
            mac_install: Some(&["brew", "install", "molten-vk"]),
            for_build: false,
        });
    }

    tools
}

// ---------------------------------------------------------------------------
// updates: the project's git checkout against its remote

#[derive(Clone, Default)]
enum Updates {
    #[default]
    Unknown,
    Checking,
    UpToDate,
    /// the new commits' subjects, newest first
    Available(Vec<String>),
    Failed(String),
}

/// fetches the remote and lists the commits the checkout doesn't have yet
fn check_updates(source: &Path) -> Updates {
    let git = |args: &[&str]| {
        quiet(Command::new("git").args(args).current_dir(source).stdin(Stdio::null()))
            .output()
            .map_err(|error| error.to_string())
            .and_then(|out| {
                if out.status.success() {
                    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
                } else {
                    Err(String::from_utf8_lossy(&out.stderr).trim().lines().last().unwrap_or("git failed").to_owned())
                }
            })
    };
    if let Err(error) = git(&["fetch", "--quiet"]) {
        return Updates::Failed(error);
    }
    match git(&["log", "--format=%s", "HEAD..@{upstream}"]) {
        Ok(log) if log.is_empty() => Updates::UpToDate,
        Ok(log) => Updates::Available(log.lines().map(str::to_owned).collect()),
        Err(error) => Updates::Failed(error),
    }
}

// ---------------------------------------------------------------------------
// reporting a problem: a new issue on GitHub, filled in with the log

/// the longest issue link: GitHub fails on ones much past 6000 characters
const MAX_ISSUE_URL: usize = 6000;

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// the log with the user's home folder hidden, as it holds their name
fn private_log(lines: &[String]) -> Vec<String> {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
    lines
        .iter()
        .map(|line| if home.len() > 3 { line.replace(&home, if cfg!(windows) { "%USERPROFILE%" } else { "~" }) } else { line.clone() })
        .collect()
}

/// a link to a new issue with the launcher's version, the tools found and
/// as much of the end of the log as fits
fn issue_url(what: &str, result: Option<bool>, log: &[String], tools: Option<&Tools>) -> String {
    let log = private_log(log);
    let mut about = vec![
        format!("- Launcher: {}", env!("CARGO_PKG_VERSION")),
        format!("- System: {} {}", std::env::consts::OS, std::env::consts::ARCH),
    ];
    if let Some(tools) = tools {
        for tool in &tools.list {
            let found = tool.found.as_deref().and_then(|f| f.lines().next()).unwrap_or("missing");
            about.push(format!("- {}: {}", tool.name, private_log(&[found.to_owned()])[0]));
        }
    }
    if !what.is_empty() {
        let state = match result {
            Some(true) => "worked",
            Some(false) => "failed",
            None => "running",
        };
        about.push(format!("- Last step: {what} ({state})"));
    }
    let body = |shown: &[String], cut: usize| {
        let note = if cut > 0 {
            format!("The first {cut} lines are cut to fit; the whole log is copied, paste it below if it helps.\n\n")
        } else {
            String::new()
        };
        let log = if shown.is_empty() {
            "(nothing ran yet)\n".to_owned()
        } else {
            format!("```\n{}\n```\n", shown.join("\n"))
        };
        format!(
            "### What happened\n\n<!-- What were you doing, and what went wrong? -->\n\n### Setup\n\n{}\n\n### Log\n\n{note}{log}",
            about.join("\n")
        )
    };
    let url = |body: &str| format!("{REPO}/issues/new?title={}&body={}", percent_encode("Problem: "), percent_encode(body));
    // drop lines from the start of the log until the link fits
    let mut cut = 0;
    loop {
        let link = url(&body(&log[cut..], cut));
        if link.len() <= MAX_ISSUE_URL || cut >= log.len() {
            return link;
        }
        cut += (log.len() - cut).div_ceil(8).max(1);
        cut = cut.min(log.len());
    }
}

// ---------------------------------------------------------------------------
// jobs: clone, update, build, each a program whose output is shown

#[derive(Default)]
struct Job {
    what: String,
    running: bool,
    /// whether the last job worked, once it's done
    result: Option<bool>,
    log: Vec<String>,
    /// the build's step, 1 to 7, and "compiled n of total" during step 4
    step: u32,
    compiled: (u32, u32),
}

impl Job {
    fn progress(&self) -> f32 {
        let compile = if self.compiled.1 > 0 { self.compiled.0 as f32 / self.compiled.1 as f32 } else { 0.0 };
        match self.step {
            0 => 0.0,
            1 | 2 => 0.02,
            3 => 0.05,
            4 => 0.10 + 0.75 * compile,
            5 => 0.87,
            6 => 0.92,
            _ => 0.99,
        }
    }

    fn take_line(&mut self, line: String) {
        if let Some(rest) = line.strip_prefix("== ") {
            if let Some(n) = rest.split('.').next().and_then(|n| n.trim().parse().ok()) {
                self.step = n;
            }
        }
        if let Some(rest) = line.trim().strip_prefix("compiled ") {
            let parts: Vec<&str> = rest.split(" of ").collect();
            if let [done, total] = parts.as_slice() {
                if let (Ok(done), Ok(total)) = (done.trim().parse(), total.trim().parse()) {
                    self.compiled = (done, total);
                }
            }
        }
        self.log.push(line);
        if self.log.len() > 4000 {
            self.log.drain(..1000);
        }
    }
}

fn run_job(job: Arc<Mutex<Job>>, ctx: egui::Context, what: &str, mut command: Command) {
    {
        let mut j = job.lock().unwrap();
        *j = Job { what: what.to_owned(), running: true, ..Default::default() };
        j.log.push(format!("> {command:?}"));
    }
    quiet(&mut command).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    std::thread::spawn(move || {
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let mut j = job.lock().unwrap();
                j.log.push(format!("could not start it: {error}"));
                j.running = false;
                j.result = Some(false);
                ctx.request_repaint();
                return;
            }
        };
        let readers: Vec<_> = [
            child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
            child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        .map(|stream| {
            let job = job.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    job.lock().unwrap().take_line(line);
                    ctx.request_repaint();
                }
            })
        })
        .collect();
        for reader in readers {
            let _ = reader.join();
        }
        let ok = child.wait().is_ok_and(|status| status.success());
        let mut j = job.lock().unwrap();
        j.running = false;
        j.result = Some(ok);
        j.log.push(if ok { "done".into() } else { "failed, see above".into() });
        ctx.request_repaint();
    });
}

// ---------------------------------------------------------------------------
// the window

struct Launcher {
    config: Config,
    rom_error: Option<String>,
    tools: Arc<Mutex<Option<Tools>>>,
    job: Arc<Mutex<Job>>,
    show_log: bool,
    message: Option<String>,
    updates: Arc<Mutex<Updates>>,
    /// what to do once the running job works: rebuild after an update
    then_build: bool,
    /// the project was updated since the game was last built
    stale: bool,
}

/// the project folder the launcher sits in, when it's run from a checkout
fn source_around_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors().find(|dir| is_source(dir) && dir.join("overrides").is_dir()).map(Path::to_path_buf)
}

fn default_source() -> PathBuf {
    match std::env::var_os("LOCALAPPDATA") {
        Some(local) => PathBuf::from(local).join("smt4-recomp").join("source"),
        None => data_dir().join("source"),
    }
}

fn is_source(dir: &Path) -> bool {
    dir.join("tools").join("build").join("Cargo.toml").is_file()
}

/// a project folder from before the build moved from build.py to tools/build
fn is_old_source(dir: &Path) -> bool {
    !is_source(dir) && dir.join("build.py").is_file()
}

impl Launcher {
    fn new(ctx: &egui::Context) -> Launcher {
        let mut config = Config::load();
        if config.source.as_deref().is_none_or(|s| !is_source(s)) {
            config.source = source_around_exe().or(config.source).or_else(|| Some(default_source()));
        }
        let rom_error = config.rom.as_deref().and_then(|rom| check_rom(rom).err());
        let launcher = Launcher {
            config,
            rom_error,
            tools: Arc::new(Mutex::new(None)),
            job: Arc::new(Mutex::new(Job::default())),
            show_log: false,
            message: None,
            updates: Arc::new(Mutex::new(Updates::Unknown)),
            then_build: false,
            stale: false,
        };
        launcher.refresh_tools(ctx);
        if launcher.config.check_updates {
            launcher.refresh_updates(ctx);
        }
        launcher
    }

    fn refresh_updates(&self, ctx: &egui::Context) {
        let source = self.source();
        if !source.join(".git").exists() {
            return;
        }
        let updates = self.updates.clone();
        let ctx = ctx.clone();
        *updates.lock().unwrap() = Updates::Checking;
        std::thread::spawn(move || {
            let found = check_updates(&source);
            *updates.lock().unwrap() = found;
            ctx.request_repaint();
        });
    }

    fn refresh_tools(&self, ctx: &egui::Context) {
        let tools = self.tools.clone();
        let ctx = ctx.clone();
        *tools.lock().unwrap() = None;
        std::thread::spawn(move || {
            *tools.lock().unwrap() = Some(check_tools());
            ctx.request_repaint();
        });
    }

    fn source(&self) -> PathBuf {
        self.config.source.clone().unwrap_or_else(default_source)
    }

    fn program(&self) -> PathBuf {
        self.source().join("target").join("release").join(EXE)
    }

    fn set_rom(&mut self, rom: PathBuf) {
        self.rom_error = check_rom(&rom).err();
        self.config.rom = Some(rom);
        self.config.save();
    }

    fn env_path(&self, tools: &Tools) -> std::ffi::OsString {
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Some(gcc) = &tools.gcc {
            dirs.extend(gcc.parent().map(Path::to_path_buf));
        }
        dirs.extend(cargo_bin());
        if cfg!(target_os = "macos") {
            dirs.extend(HOMEBREW_BINS.map(PathBuf::from));
        }
        dirs.extend(std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default());
        std::env::join_paths(dirs).unwrap_or_default()
    }

    fn start_build(&mut self, ctx: &egui::Context, tools: &Tools) {
        let (Some(rom), Some(gcc)) = (&self.config.rom, &tools.gcc) else { return };
        let ar = gcc.with_file_name(if cfg!(windows) { "ar.exe" } else { "ar" });
        let cargo = cargo_bin()
            .map(|bin| bin.join(if cfg!(windows) { "cargo.exe" } else { "cargo" }))
            .filter(|cargo| cargo.is_file())
            .unwrap_or_else(|| PathBuf::from("cargo"));
        let mut command = Command::new(cargo);
        command
            .args(["run", "--release", "--manifest-path"])
            .arg(Path::new("tools").join("build").join("Cargo.toml"))
            .arg("--")
            .arg(rom)
            .arg("--low-priority")
            .current_dir(self.source())
            .env("CC", gcc)
            .env("AR", ar)
            .env("PATH", self.env_path(tools));
        run_job(self.job.clone(), ctx.clone(), "Building the game", command);
    }

    fn start_clone(&mut self, ctx: &egui::Context) {
        let dir = self.source();
        if let Some(parent) = dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut command = Command::new("git");
        command.args(["clone", "--depth", "1", REPO]).arg(&dir);
        run_job(self.job.clone(), ctx.clone(), "Downloading the project", command);
    }

    fn start_update(&mut self, ctx: &egui::Context) {
        let mut command = Command::new("git");
        command.args(["pull", "--ff-only"]).current_dir(self.source());
        run_job(self.job.clone(), ctx.clone(), "Updating the project", command);
    }

    fn play(&mut self) {
        let (Some(rom), program) = (&self.config.rom, self.program()) else { return };
        let started = Command::new(&program).arg(rom).current_dir(program.parent().unwrap_or(Path::new("."))).spawn();
        self.message = Some(match started {
            Ok(_) => "The game is starting.".into(),
            Err(error) => format!("Could not start {}: {error}", program.display()),
        });
    }
}

fn status(ui: &mut egui::Ui, ok: bool, text: &str) {
    let color = if ok { egui::Color32::from_rgb(90, 200, 120) } else { egui::Color32::from_rgb(230, 90, 90) };
    ui.horizontal(|ui| {
        // a dot, as the default fonts have no check mark
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.5, color);
        ui.add(egui::Label::new(text).truncate());
    });
}

/// a section of the window, as wide as the window
fn section(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
}

impl eframe::App for Launcher {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        let tools = self.tools.lock().unwrap().clone();
        let (running, job_what, job_result, progress, step) = {
            let j = self.job.lock().unwrap();
            (j.running, j.what.clone(), j.result, j.progress(), j.step)
        };
        if !running && self.then_build {
            self.then_build = false;
            if job_result == Some(true) {
                *self.updates.lock().unwrap() = Updates::UpToDate;
                self.stale = true;
                if let Some(tools) = tools.as_ref().filter(|t| t.ok()) {
                    self.start_build(ctx, tools);
                    self.stale = false;
                }
            }
        }
        let updates = self.updates.lock().unwrap().clone();
        let source_ok = is_source(&self.source());
        let rom_ok = self.config.rom.is_some() && self.rom_error.is_none();
        let built = self.program().is_file();

        egui::CentralPanel::default().show(root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Shin Megami Tensei IV for PC");
                ui.label("A native build of the 3DS game, made from your own copy. Nothing it builds may be shared.");
                ui.add_space(8.0);

                // 1. the game
                section(ui, |ui| {
                    ui.strong("1. Your game");
                    match &self.config.rom {
                        Some(rom) => {
                            status(ui, rom_ok, &rom.display().to_string());
                            if let Some(error) = &self.rom_error {
                                ui.colored_label(egui::Color32::from_rgb(230, 90, 90), error);
                            }
                        }
                        None => {
                            ui.label("Choose the folder with your decrypted Shin Megami Tensei IV (USA) .3ds, or the file.");
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!running, egui::Button::new("Choose the folder…")).clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                match find_rom(&folder) {
                                    Some(rom) => self.set_rom(rom),
                                    None => {
                                        self.message = Some(format!(
                                            "No decrypted Shin Megami Tensei IV (USA) in {}.",
                                            folder.display()
                                        ))
                                    }
                                }
                            }
                        }
                        if ui.add_enabled(!running, egui::Button::new("Choose the file…")).clicked() {
                            if let Some(file) = rfd::FileDialog::new().add_filter("3DS game", &["3ds", "cci", "cxi"]).pick_file() {
                                self.set_rom(file);
                            }
                        }
                    });
                });

                // 2. tools
                section(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("2. Tools for the build");
                        if ui.add_enabled(!running, egui::Button::new("Check again")).clicked() {
                            self.refresh_tools(ctx);
                        }
                    });
                    match &tools {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Checking…");
                            });
                        }
                        Some(tools) => {
                            for tool in &tools.list {
                                ui.horizontal(|ui| {
                                    status(ui, tool.found.is_some(), tool.name);
                                    match &tool.found {
                                        Some(found) => {
                                            ui.add(egui::Label::new(egui::RichText::new(found.lines().next().unwrap_or("")).weak()).truncate());
                                        }
                                        None => {
                                            ui.weak(&tool.hint);
                                        }
                                    }
                                });
                                if tool.found.is_none() {
                                    ui.horizontal(|ui| {
                                        ui.add_space(20.0);
                                        if tool.name == "Rust" && tools.rust_msvc {
                                            if ui.button("Switch to the GNU toolchain").clicked() {
                                                let mut command = Command::new("rustup");
                                                command.args(["default", "stable-x86_64-pc-windows-gnu"]);
                                                run_job(self.job.clone(), ctx.clone(), "Switching Rust to GNU", command);
                                            }
                                        } else if let Some(install) = tool.mac_install.filter(|_| cfg!(target_os = "macos")) {
                                            if ui.add_enabled(!running, egui::Button::new(format!("Install ({})", install.join(" ")))).clicked() {
                                                let mut command = Command::new(install[0]);
                                                command.args(&install[1..]).env("PATH", self.env_path(tools));
                                                run_job(self.job.clone(), ctx.clone(), &format!("Installing {}", tool.name), command);
                                                self.message = Some(format!(
                                                    "Installing {}. When it's done, press Check again (Rust may need the launcher reopened).",
                                                    tool.name
                                                ));
                                            }
                                        } else if let Some(id) = tool.winget {
                                            if cfg!(windows) && ui.button(format!("Install ({id})")).clicked() {
                                                // in a console of its own, so its prompts show
                                                let _ = Command::new("cmd")
                                                    .args(["/c", "start", "winget", "install", "-e", "--id", id])
                                                    .spawn();
                                                self.message = Some(format!(
                                                    "Installing {}. When it's done, press Check again (Rust may need the launcher reopened).",
                                                    tool.name
                                                ));
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                });

                // 3. source
                section(ui, |ui| {
                    ui.strong("3. Project source");
                    status(ui, source_ok, &self.source().display().to_string());
                    ui.horizontal(|ui| {
                        let git = tools.as_ref().is_some_and(|t| t.list.first().is_some_and(|g| g.found.is_some()));
                        let old = is_old_source(&self.source());
                        let cloned = self.source().join(".git").exists();
                        if old && !cloned {
                            ui.weak("This folder is an older version of the project, which built with Python. Download a newer release, or choose an empty folder and press Download.");
                        } else if old {
                            ui.weak("An older version of the project: press Update.");
                            if ui.add_enabled(!running && git, egui::Button::new("Update")).clicked() {
                                self.start_update(ctx);
                            }
                        } else if !source_ok {
                            if ui.add_enabled(!running && git, egui::Button::new("Download")).clicked() {
                                self.start_clone(ctx);
                            }
                        } else if cloned {
                            match &updates {
                                Updates::Checking => {
                                    ui.spinner();
                                    ui.label("Looking for updates…");
                                }
                                Updates::UpToDate => {
                                    ui.weak("Up to date.");
                                }
                                Updates::Available(_) => {}
                                Updates::Failed(error) => {
                                    ui.weak(format!("Couldn't look for updates: {error}"));
                                }
                                Updates::Unknown => {}
                            }
                            if !matches!(updates, Updates::Available(_) | Updates::Checking)
                                && ui.add_enabled(!running && git, egui::Button::new("Check for updates")).clicked()
                            {
                                self.refresh_updates(ctx);
                            }
                        } else {
                            ui.weak("Not a git download, so Update can't refresh it; download a newer release to update.");
                        }
                        if ui.add_enabled(!running, egui::Button::new("Choose another folder…")).clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                self.config.source = Some(folder);
                                self.config.save();
                                *self.updates.lock().unwrap() = Updates::Unknown;
                                if self.config.check_updates {
                                    self.refresh_updates(ctx);
                                }
                            }
                        }
                    });
                    if let Updates::Available(commits) = &updates {
                        ui.add_space(4.0);
                        let what = if commits.len() == 1 { "1 update".to_owned() } else { format!("{} updates", commits.len()) };
                        ui.colored_label(egui::Color32::from_rgb(240, 190, 80), format!("{what} available:"));
                        for subject in commits.iter().take(8) {
                            ui.add(egui::Label::new(format!("• {subject}")).truncate());
                        }
                        if commits.len() > 8 {
                            ui.weak(format!("and {} more", commits.len() - 8));
                        }
                        let label = if built { "Update and rebuild" } else { "Update" };
                        let git = tools.as_ref().is_some_and(|t| t.list.first().is_some_and(|g| g.found.is_some()));
                        if ui.add_enabled(!running && git, egui::Button::new(label)).clicked() {
                            self.start_update(ctx);
                            self.then_build = built;
                        }
                    }
                    if self.stale && !running {
                        ui.weak("The project was updated: press Rebuild to use it.");
                    }
                    if ui.checkbox(&mut self.config.check_updates, "Look for updates when the launcher opens").changed() {
                        self.config.save();
                    }
                });

                // 4. build and play
                section(ui, |ui| {
                    ui.strong("4. Build and play");
                    let tools_ok = tools.as_ref().is_some_and(Tools::ok);
                    let can_build = !running && rom_ok && tools_ok && source_ok;
                    if built && !running {
                        status(ui, true, "The game is built.");
                    } else if !running {
                        ui.label("The first build takes 20 minutes or more. You can keep using the computer meanwhile.");
                    }
                    ui.horizontal(|ui| {
                        if built {
                            let play = egui::Button::new(egui::RichText::new("  Play  ").size(20.0));
                            if ui.add_enabled(!running && rom_ok, play).clicked() {
                                self.play();
                            }
                            if ui.add_enabled(can_build, egui::Button::new("Rebuild")).clicked() {
                                if let Some(tools) = &tools {
                                    self.start_build(ctx, tools);
                                }
                            }
                        } else {
                            let build = egui::Button::new(egui::RichText::new("  Build the game  ").size(20.0));
                            if ui.add_enabled(can_build, build).clicked() {
                                if let Some(tools) = &tools {
                                    self.start_build(ctx, tools);
                                }
                            }
                        }
                    });
                    if !running && !can_build && !built {
                        let missing: Vec<&str> = [
                            (!rom_ok, "your game"),
                            (!tools_ok, "the tools"),
                            (!source_ok, "the project source"),
                        ]
                        .into_iter()
                        .filter_map(|(m, what)| m.then_some(what))
                        .collect();
                        if !missing.is_empty() {
                            ui.weak(format!("Needs {} first.", missing.join(", ")));
                        }
                    }
                    if running || job_result.is_some() {
                        ui.add_space(4.0);
                        if running {
                            let text = if job_what == "Building the game" && step > 0 {
                                let steps = ["", "tools", "ROM", "recompiler", "recompiling the game", "patching", "building the program", "cheats"];
                                format!("{job_what}: {}", steps.get(step as usize).unwrap_or(&""))
                            } else {
                                job_what.clone()
                            };
                            let bar = if job_what == "Building the game" {
                                egui::ProgressBar::new(progress).show_percentage().text(text)
                            } else {
                                egui::ProgressBar::new(0.0).animate(true).text(text)
                            };
                            ui.add(bar);
                        } else if let Some(ok) = job_result {
                            status(ui, ok, &format!("{job_what}: {}", if ok { "done" } else { "failed, see the log" }));
                        }
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_log, "Show the log");
                            if ui.button("Copy the log").clicked() {
                                let text = self.job.lock().unwrap().log.join("\n");
                                ui.ctx().copy_text(text);
                                self.message = Some(if cfg!(target_os = "macos") {
                                    "The log is copied: paste it in the issue with Cmd+V."
                                } else {
                                    "The log is copied: paste it in the issue with Ctrl+V."
                                }.into());
                            }
                        });
                    }
                });

                if let Some(message) = &self.message {
                    ui.add_space(4.0);
                    ui.label(message);
                }

                if self.show_log {
                    ui.add_space(4.0);
                    let job = self.job.lock().unwrap();
                    egui::ScrollArea::vertical().id_salt("log").max_height(220.0).stick_to_bottom(true).show(ui, |ui| {
                        for line in job.log.iter().rev().take(400).rev() {
                            ui.monospace(line);
                        }
                    });
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.hyperlink_to("Project page", REPO);
                    ui.label("·");
                    let report = ui.link("Report a problem").on_hover_text("Opens a new issue on GitHub with the log filled in");
                    if report.clicked() {
                        let (url, whole) = {
                            let job = self.job.lock().unwrap();
                            (issue_url(&job.what, job.result, &job.log, tools.as_ref()), private_log(&job.log).join("\n"))
                        };
                        if !whole.is_empty() {
                            ui.ctx().copy_text(whole);
                        }
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        self.message = Some(
                            "A new issue is open in your browser with the log. Describe what happened, then press Create.".into(),
                        );
                    }
                });
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_links_fit_and_hide_the_home_folder() {
        assert_eq!(percent_encode("a b/é\n"), "a%20b%2F%C3%A9%0A");
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap();
        let log: Vec<String> = (0..3000).map(|i| format!("line {i} in {home}\\dev")).collect();
        let url = issue_url("Building the game", Some(false), &log, None);
        assert!(url.len() <= MAX_ISSUE_URL, "{}", url.len());
        assert!(url.contains("line%202999"), "the end of the log is kept");
        assert!(!url.contains(&percent_encode(&home)), "the home folder is hidden");
        assert!(url.contains(if cfg!(windows) { "%25USERPROFILE%25" } else { "~%5Cdev" }));
        let short = issue_url("", None, &[], None);
        assert!(short.contains("nothing%20ran%20yet"));
    }

    /// a checkout behind its remote by two commits sees them, then none once pulled
    #[test]
    fn finds_new_commits() {
        let dir = std::env::temp_dir().join(format!("smt4-launcher-updates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |cwd: &Path, args: &[&str]| {
            let ok = Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main"])
                .args(args)
                .current_dir(cwd)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "git {args:?}");
        };
        let (remote, ours, theirs) = (dir.join("remote.git"), dir.join("ours"), dir.join("theirs"));
        git(&dir, &["init", "--bare", "remote.git"]);
        git(&dir, &["clone", "-q", "remote.git", "theirs"]);
        git(&theirs, &["commit", "-q", "--allow-empty", "-m", "first"]);
        git(&theirs, &["push", "-q", "origin", "HEAD:main"]);
        git(&dir, &["clone", "-q", "--depth", "1", &format!("file://{}", remote.display()), "ours"]);
        assert!(matches!(check_updates(&ours), Updates::UpToDate));
        git(&theirs, &["commit", "-q", "--allow-empty", "-m", "second"]);
        git(&theirs, &["commit", "-q", "--allow-empty", "-m", "third"]);
        git(&theirs, &["push", "-q", "origin", "HEAD:main"]);
        match check_updates(&ours) {
            Updates::Available(commits) => assert_eq!(commits, ["third", "second"]),
            _ => panic!("no updates seen"),
        }
        git(&ours, &["pull", "-q", "--ff-only"]);
        assert!(matches!(check_updates(&ours), Updates::UpToDate));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// SMT4_ROM_DIR=<folder with the game> cargo test
    #[test]
    fn finds_the_rom_in_a_folder() {
        let Some(dir) = std::env::var_os("SMT4_ROM_DIR") else { return };
        let dir = PathBuf::from(dir);
        let rom = find_rom(&dir).expect("no ROM found");
        assert!(check_rom(&rom).is_ok());
        // and from the folder above it
        assert_eq!(find_rom(dir.parent().unwrap()), Some(rom));
    }
}
