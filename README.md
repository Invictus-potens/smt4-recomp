# smt4-recomp

A static recompilation project for *Shin Megami Tensei IV* (3DS, USA,
title ID `00040000000E5C00`), built on top of
[3dsrecomp](https://github.com/fearkov/3dsrecomp) and
[Zakuro](https://github.com/fearkov/zakuro).

This is **not** a decompilation. It does not reconstruct readable,
matching source code. 3dsrecomp translates the game's compiled ARM11
code directly into generated C ahead of time, which runs natively
instead of being interpreted. This repository holds the glue around
that process for this specific title: the standalone program project
(from `3dsrecomp port`), hand-written overrides for functions that
need fixing, community cheat codes, and build/progress notes.

## What's here

- `Cargo.toml`, `build.rs`, `src/main.rs`: the standalone program
  project `3dsrecomp port` generates, which links the recompiled code
  against Zakuro as the runtime (kernel, services, GPU, sound).
- `overrides/`: hand-written C functions that replace generated ones,
  for fixes or mods. See [3dsrecomp's docs on overrides](
  https://github.com/fearkov/3dsrecomp/blob/main/docs/overrides.md).
  `multipliers.c` holds the Macca, hero EXP, demon EXP and App Points multipliers.
- `scripts/patch_generated.py`: patches the generated C at the few spots
  an override can't reach (they aren't function entries). Run after
  every `port`.
- `cheats/00040000000E5C00.txt`: Action Replay / Gateway codes in the
  format Citra/Azahar/Zakuro read: community codes, plus the multiplier
  switches (`Macca x2`, `Hero EXP x5`, `Demon EXP x3`, …) that only do something in this
  build.
- `docs/PROGRESS.md`: build status, code coverage, known issues.
- `docs/MULTIPLIERS.md`: how the multipliers work and how each hook was
  found.
- `docs/ENHANCEMENTS.md`: ideas: screens, widescreen, cheats.

## What's NOT here, and never will be

- The ROM.
- The generated C (`code/`, ~195 MiB, derived straight from the
  copyrighted game binary).
- Any compiled library or executable.

You need your own legally dumped copy of the game to build anything in
this repo. See [3dsrecomp's README](
https://github.com/fearkov/3dsrecomp#readme) for how dumping and
building work.

## Building

You need Rust, a C compiler and `ar` (MinGW-w64's on Windows, matching
whatever Rust toolchain you use; see `3dsrecomp port`'s docs for the
Windows toolchain-matching caveat), and Python 3.

```
3dsrecomp port <path to your rom> <scratch dir> --name smt4 --overrides overrides/
# move <scratch dir>/code here, replacing code/
python scripts/patch_generated.py --rebuild
cargo build --release
target/release/smt4 <path to your rom>
```

Port into a scratch directory and keep only its `code/`: `port` also
writes its own `Cargo.toml` and `src/main.rs`, which would drop this
project's (`main.rs` defines `smt4_log_override`, which the overrides
call). `code/` is kept out of version control by `.gitignore`. Redo these
steps whenever you pull changes to `overrides/` or
`scripts/patch_generated.py`.

Copy `cheats/00040000000E5C00.txt` to `%APPDATA%\zakuro\cheats\` (or the
equivalent on your system) to get the cheats, multipliers included, in
Zakuro's cheat menu. Set `SMT4_HOOK_LOG` to a file path to have every
multiplied gain written to it.

## Status

See [docs/PROGRESS.md](docs/PROGRESS.md).
