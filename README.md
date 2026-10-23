<p align="center">
  <img src="https://github.com/user-attachments/assets/4ae70845-a934-4430-9e5b-c35545c04156" alt="Shin Megami Tensei IV" width="117" height="108">
</p>

# Shin Megami Tensei IV for PC (smt4-recomp)

**English** | [Português (Brasil)](README.pt-BR.md)

A native Windows and macOS build of *Shin Megami Tensei IV* (3DS, USA), made by static
recompilation: [3dsrecomp](https://github.com/fearkov/3dsrecomp) translates
the game's ARM code into C ahead of time, and that C is compiled into a
program of its own, with [Zakuro](https://github.com/fearkov/zakuro) as the
runtime for everything else (kernel, services, GPU, sound). The game's code
runs natively, not through an emulator's interpreter or JIT.

On top of that, this project adds widescreen, multipliers for Macca, EXP and
App Points, frame rate options and cheats, all switched from the in-game
cheat menu.

> **Beta.** Played from a new game for more than 10 hours with no crash.
> The rest of the game hasn't been played on it yet: please report what you
> find in [Issues](../../issues).

> **No game files or game code are included, and there is no download.** You
> build it yourself from your own dump of the game. What the build makes
> (`code/` and the program) holds the game's code: don't share it.
> This project is not affiliated with Atlus, SEGA or Nintendo.

## Contents

- [Features](#features)
- [Requirements](#requirements)
- [Building](#building)
- [Playing](#playing)
- [Cheats and enhancements](#cheats-and-enhancements)
- [Controls](#controls)
- [Extracting assets](#extracting-assets)
- [Troubleshooting](#troubleshooting)
- [How it works](#how-it-works)
- [Credits](#credits)
- [License](#license)

## Features

- **Native code**: the game's ARM code recompiled to C and compiled for the
  PC (95% of it found ahead of time; anything left runs in Zakuro's
  interpreter).
- **Widescreen** 16:9 and 21:9: the game draws a wider 3D scene (more view
  at the sides, same height), not a stretched picture.
- **Multipliers** x2, x3 and x5 for Macca, hero EXP, demon EXP and App
  Points, each picked from the cheat menu while playing.
- **Easy Recruit**: demon negotiations always go your way, from the cheat
  menu.
- **60 FPS** (the game runs twice as fast with it) and **30 FPS** to go back.
- Community cheats: max Macca, App Points, items, hero EXP, HP/MP and stats.
- Everything Zakuro has: Vulkan (Metal through MoltenVK on macOS) or
  OpenGL, screen layouts, scaling and filters, save states, controllers, 3D.

## Requirements

- Windows 10 or 11, 64-bit. Linux should work too, but hasn't been tried.
- A graphics card with Vulkan (or OpenGL 3.3).
- **Your own decrypted dump** of *Shin Megami Tensei IV* (USA), title ID
  `00040000000E5C00`, as `.3ds` or `.cci`.
- About 10 GB of free disk space and 8 GB of RAM for the build.
- To build:
  - [Rust](https://rustup.rs) with the GNU toolchain:
    `rustup default stable-x86_64-pc-windows-gnu`
  - MinGW-w64 GCC, 64-bit: `winget install BrechtSanders.WinLibs.POSIX.UCRT`
  - [git](https://git-scm.com)

### macOS

- A Mac with Apple Silicon (M1 or newer). Intel Macs haven't been tried.
- Apple's command line tools (clang, ar and git): `xcode-select --install`
- [Rust](https://rustup.rs), the default toolchain
- [MoltenVK](https://github.com/KhronosGroup/MoltenVK), Vulkan on Metal, which
  the game draws with: `brew install molten-vk` ([Homebrew](https://brew.sh)).
  The macOS build has no OpenGL.

## Building

### With the launcher

`launcher/` is a small program that does it all from a window: choose the
folder with your game (it finds the ROM in it), see which tools are missing
and install them with winget, download or update this project, build the game
with a progress bar, and play.

```
cd launcher
cargo build --release
target\release\smt4-launcher.exe
```

On macOS it's `target/release/smt4-launcher`, and it installs the tools with
`xcode-select --install`, rustup and `brew install molten-vk` instead.

It holds no game code, so it can be shared.

### From the command line

```
git clone https://github.com/Invictus-potens/smt4-recomp
cd smt4-recomp
cargo build-game "C:\path\to\Shin Megami Tensei IV (USA).3ds"
```

`cargo build-game` (the builder in `tools/build`) checks your tools and ROM, fetches and builds 3dsrecomp at the
version this project was tested with, recompiles the game's code with this
project's changes, builds the program and copies the cheats to Zakuro's
cheats folder (adding the missing ones to a cheats file already there,
without touching the rest). The recompilation takes **20 minutes or more**; add
`--low-priority` to keep using the PC meanwhile. Running it again later only
redoes what changed. `cargo build-game --help` lists the options.

When it's done, the program is `target\release\smt4.exe`.

On macOS it's the same with Unix paths, and the program is
`target/release/smt4`:

```
cargo build-game ~/Games/"Shin Megami Tensei IV (USA).3ds"
```

Add `--bundle` to also make `target/release/SMT4.app`, which opens from the
Finder with the ROM it was built from (or one dropped on it) and carries
MoltenVK inside, so it doesn't need Homebrew to run. Like the program, it
holds the game's code: don't share it.

## Playing

```
target\release\smt4.exe "C:\path\to\Shin Megami Tensei IV (USA).3ds"
```

It reads the game's files and everything but the code from your dump, so
keep the ROM where it is. Saves, save states and settings go in Zakuro's
folder, `%APPDATA%\zakuro` (on macOS, `~/Library/Application Support/zakuro`).

Recommended setup (Escape opens the menu):

1. **Settings**, Layout: **Top screen only**. F10 switches to the bottom
   screen and back, for the map and menus.
2. **Settings**, Top screen shape: **16:9** (or 21:9 on an ultrawide).
3. **Cheats**: turn on **Widescreen 16:9** (the same shape), then go to
   another area so the game sets its camera again.

## Cheats and enhancements

Escape, then **Cheats**. The ones marked *smt4-recomp* only work in this
build; *Enhancements* are built into Zakuro, at the top of the window; the
others are standard Action Replay codes.

| Cheat | What it does |
|---|---|
| Widescreen 16:9 / 21:9 *(smt4-recomp)* | Wider 3D scene. Needs Top screen shape set to the same, and shows at the next area change. |
| Macca x2 / x3 / x5 *(smt4-recomp)* | Multiplies Macca from battles, shops and rewards. Shops show the normal price; the money you get is multiplied. |
| Hero EXP x2 / x3 / x5 *(smt4-recomp)* | Multiplies the hero's EXP from battles. |
| Demon EXP x2 / x3 / x5 *(smt4-recomp)* | Multiplies the EXP demons get (party and stock). |
| App Points x2 / x3 / x5 *(smt4-recomp)* | Multiplies App Points. |
| Easy Recruit *(smt4-recomp)* | Every negotiation goes your way: answers work, refusing or not having what it asks still works, and the demon doesn't run off or cheat you. It still asks for things. |
| 60 FPS *(Enhancements)* | The game logic is tied to the frame rate, so everything runs **twice as fast**. Turning it off does **not** bring the normal speed back: use 30 FPS for that. |
| 30 FPS *(Enhancements)* | The game's default; undoes 60 FPS. |
| Max Macca, 9999 App points, All items, Hero Max Exp, Hero HP/MP, Hero Stats | Community codes. |

Turn on only one of each multiplier. Turning a multiplier or widescreen
cheat off goes back to normal on its own.

How they work: [MULTIPLIERS.md](docs/MULTIPLIERS.md),
[WIDESCREEN.md](docs/WIDESCREEN.md), [RECRUIT.md](docs/RECRUIT.md).

## Controls

Keyboard defaults, all changeable in Settings. Controllers work too.

| 3DS | Key | | Zakuro | Key |
|---|---|---|---|---|
| A / B | X / Z | | Menu | Escape |
| X / Y | S / A | | Pause | F1 |
| L / R | Q / W | | Save / load state | F5 / F8 |
| ZL / ZR | 1 / 2 | | Next layout | F9 |
| Start / Select | Enter / Backspace | | Top / bottom screen | F10 |
| D-pad | Arrow keys | | Full screen | F11 |
| Circle Pad | I J K L | | Fast forward | hold Tab |
| C-Stick | T F G H | | Touch screen | mouse |

On a Mac keyboard the F keys need Fn held, unless they're set as standard
function keys in System Settings, Keyboard.

## Extracting assets

`tools/extract` pulls the game's files out of your ROM and converts them:
textures to PNG, models to plain CGFX (for SPICA, Ohana3DS or Noesis), text
to UTF-8, music, sounds and voices to WAV, cutscenes to MP4.

```
cd tools\extract
cargo build --release
target\release\smt4-extract "C:\path\to\Shin Megami Tensei IV (USA).3ds" C:\smt4-assets
```

The formats, what's converted and what isn't: [ASSETS.md](docs/ASSETS.md).
Like the build, what it writes is the game's: don't share it.

To replace an image of the game with your own (a portrait, an icon, a
background), at the same resolution or higher:

```
cargo fit-texture <assets>\textures\tex\stex\stbustup\st_bu_pc001.png my_picture.png
```

It fits your picture where the original's character is and installs it as a
texture pack; restart the game to see it. Step by step, and what to do when
an image doesn't change: [MODDING.md](docs/MODDING.md).

## Troubleshooting

- **"no 64-bit gcc found"**: only an old 32-bit MinGW is installed (one that
  comes first on PATH is skipped on its own). Install MinGW-w64, for example
  `winget install BrechtSanders.WinLibs.POSIX.UCRT`, or set `CC` and `AR` to
  the full paths of a 64-bit `gcc.exe` and `ar.exe`.
- **`.drectve ... unrecognized` warnings, then a link error**: the gcc that
  comes with Strawberry Perl has a linker too old for current Rust. The build
  skips it on its own when another 64-bit gcc is installed; install one with
  `winget install BrechtSanders.WinLibs.POSIX.UCRT`.
- **"Rust uses the MSVC toolchain"**, or errors about `link.exe`: run
  `rustup default stable-x86_64-pc-windows-gnu`.
- **"is title ..., this project is for Shin Megami Tensei IV (USA)"**: only
  the USA release is supported; the code differs between regions.
- **The game runs twice as fast**: 60 FPS was on at some point. Turn on
  **30 FPS** (Enhancements), then save a state again.
- **The picture is squeezed or stretched**: the Widescreen cheat and the Top
  screen shape setting have to match, or both be off.
- **macOS: "Could not start Vulkan"**: MoltenVK isn't installed, run
  `brew install molten-vk`. A MoltenVK somewhere else can be named with
  `ZAKURO_VULKAN_LIB=/path/to/libMoltenVK.dylib`.
- **macOS: a window asking to install the developer tools**: the build looked
  for clang or git; install them there, or with `xcode-select --install`.

## How it works

| Part | Where | What it does |
|---|---|---|
| Launcher | `launcher/` | A window around `tools/build`: finds the ROM, checks the tools, builds, plays |
| Builder | `tools/build` (`cargo build-game`) | Runs every step below, from your ROM to the program |
| Recompiler | [3dsrecomp](https://github.com/fearkov/3dsrecomp) | Finds the game's functions and writes them as C, into `code/` (not in this repo) |
| Overrides | `overrides/` | Functions written by hand that replace generated ones: the Macca and App Points multipliers, widescreen |
| Patches | `tools/build/src/patch.rs` | Changes the generated C where an override can't reach (hero and demon EXP, the projection) |
| Program | `Cargo.toml`, `build.rs`, `src/main.rs` | Links the compiled code with Zakuro |
| Runtime | [Zakuro](https://github.com/fearkov/zakuro) | 3DS kernel, services, GPU and sound |
| Image mods | `tools/fit-texture` (`cargo fit-texture`) | Makes and installs a texture pack picture, see [MODDING.md](docs/MODDING.md) |
| Asset extractor | `tools/extract/` | Pulls the game's files out of the ROM and converts them, see [ASSETS.md](docs/ASSETS.md) |
| Cheats | `cheats/00040000000E5C00.txt` | Codes for Zakuro's cheat menu; the *smt4-recomp* ones write a control byte the overrides read |

This is static recompilation, not a decompilation: there is no readable
source code of the game here or anywhere in the build. Set `SMT4_HOOK_LOG`
to a file path to have every multiplied gain written to it.

More: [progress and known issues](docs/PROGRESS.md),
[enhancements and roadmap](docs/ENHANCEMENTS.md).

## Credits

- **fearkov**: [3dsrecomp](https://github.com/fearkov/3dsrecomp) and
  [Zakuro](https://github.com/fearkov/zakuro), which do the heavy lifting.
- **lucky**: the 60 FPS and 30 FPS codes, from
  [Reshiban/60FPS-AR-CHEATS-3DS](https://github.com/Reshiban/60FPS-AR-CHEATS-3DS).
- **JourneyOver** and the codes' original authors: the community cheats,
  from [JourneyOver/CTRPF-AR-CHEAT-CODES](https://github.com/JourneyOver/CTRPF-AR-CHEAT-CODES).
- *Shin Megami Tensei IV* is by Atlus.

## License

[MIT](LICENSE), for the files in this repository only. It doesn't cover the
game or anything built from it.
