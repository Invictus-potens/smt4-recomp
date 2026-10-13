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

- `Cargo.toml`, `build.rs`, `src/main.rs` — the standalone program
  project `3dsrecomp port` generates, which links the recompiled code
  against Zakuro as the runtime (kernel, services, GPU, sound).
- `overrides/` — hand-written C functions that replace generated ones,
  for fixes or mods. See [3dsrecomp's docs on overrides](
  https://github.com/fearkov/3dsrecomp/blob/main/docs/overrides.md).
- `cheats/00040000000E5C00.txt` — community Action Replay / Gateway
  codes, in the format Citra/Azahar/Zakuro read.
- `docs/PROGRESS.md` — build status, code coverage, known issues.

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

You need Rust and a C compiler (MinGW-w64's gcc on Windows, matching
whatever Rust toolchain you use — see `3dsrecomp port`'s docs for the
Windows toolchain-matching caveat).

```
3dsrecomp port <path to your rom> . --name smt4 --overrides overrides/
cargo build --release
target/release/smt4 <path to your rom>
```

The `port` step regenerates `code/`, which this repo's `.gitignore`
keeps out of version control. Re-run it whenever you pull changes that
touch the generated code's inputs.

## Status

See [docs/PROGRESS.md](docs/PROGRESS.md).
