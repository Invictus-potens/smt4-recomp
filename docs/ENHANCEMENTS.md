# Enhancements roadmap

Ideas for improving the SMT4 experience on top of the stock recomp,
roughly ordered by effort.

## 1. Split/separate screens — already done, zero code

Zakuro ships 5 screen layouts natively (`Screens` enum in
`zakuro/src/settings.rs`, backed by `ScreenLayout` in `zakuro-gpu`):

| Layout | What it does |
|---|---|
| Top over bottom (default) | matches the console |
| Side by side | both screens next to each other |
| **Top screen only** | hides the bottom (touch) screen entirely |
| Bottom screen only | the reverse, rarely useful |
| **Big top, small bottom** | top screen large, bottom shrunk to a corner |

Cycle with **F10**, or set permanently in the in-game Settings menu
(gear icon, under Graphics). `Top screen only` or `Big top screen`
are the two worth using for SMT4 — the bottom screen is only a
menu/status display outside of a few touch-specific UI bits (naming,
the Gauntlet Hunt log), so losing it costs little.

No work needed here beyond telling people the option exists.

## 2. True widescreen — real work, no shortcut found

What "Top screen only" does NOT do: it doesn't change the rendered
aspect ratio. The 3D scene is still composed for the 3DS's 400×240
(5:3) top screen and just gets letterboxed/pillarboxed to fit a 16:9
window. Actual widescreen needs the game's own camera/projection code
patched to render more horizontally — exactly what TriAevum did for
Ocarina of Time 3D with a custom renderer, and what the N64/GameCube
recomp scene does by rewriting the camera entirely.

Searched for an existing Action Replay / Gateway widescreen code for
SMT4 specifically — found none. Widescreen AR codes are rare for 3DS
titles in general (unlike GBA/DS where screen-stretch hacks are
common), because the projection matrix is usually computed from
runtime values in a way a memory-write-only cheat code can't patch
reliably across every frame.

Realistic paths, hardest to easiest:
- **Find and override the camera/projection function.** Use the
  generated C + a disassembler (IDA/Ghidra) to find where the
  perspective matrix is built (look for a function writing to the
  PICA200 `PERSPECTIVE_PROJ` floats — it'll take an aspect ratio
  constant, probably `4:3`-ish, 400.0/240.0). Write a
  `RECOMP_OVERRIDE` (see `docs/overrides.md` in 3dsrecomp) that
  patches the aspect ratio before calling through to the original.
  This is the only way to get genuinely more horizontal view, not
  just a stretched image. Needs real reverse-engineering time.
- **Cheap stretch, not true widescreen.** A cheat or override that
  scales the final framebuffer non-uniformly (stretches the existing
  4:3 render to 16:9) distorts character models — visually worse
  than letterboxing for a game like this, not worth doing.
- **Ask upstream.** Worth a Discord question to fearkov/the Zakuro
  community — this could become a general Zakuro feature (home-grown
  FOV widening per title, similar to Citra/Azahar's old "Stretch to
  Window" vs actual 3D hacks some communities maintain per game) that
  benefits every title, not just SMT4.

Given the effort, this is a "maybe later" item, not a quick win.

## 3. More / different cheats

Added so far (`cheats/00040000000E5C00.txt`, all from the community,
Citra/Gateway AR format which Zakuro reads natively):

- Max Macca, 9999 App Points, All Items
- Hero Max Exp, HP/MP, Stats
- **60 FPS** (single write at `204E3BA0`, by @lucky)

Possible additions, roughly by how useful they'd be:
- **No random encounters / instant kill** — common quality-of-life
  codes for grinding-heavy Atlus RPGs; worth searching
  gamegenie.53lu.com / GBAtemp's SMT4 cheat thread for codes beyond
  what JourneyOver's repo has.
- **Demon compendium codes** (unlock/max all demons) — the GBAtemp
  cheat-table thread (LuckyCross) made per-demon AR codes for Cheat
  Engine; porting the useful ones to the Gateway/AR text format Zakuro
  reads is mechanical but tedious (dozens of demons).
- **60 FPS as a Zakuro "enhancement"** — SMT4 isn't in Zakuro's
  built-in `enhancements.txt` list yet (checked: Pokémon, Mario 3D
  Land, Tomodachi Life and a few others are, SMT4 isn't). Submitting
  our 60 FPS find there as a small upstream PR would let anyone turn
  it on by name from Zakuro's Cheats menu instead of needing our
  cheat file — low effort, benefits the whole community. Needs the
  cart's `rev` (remaster_version) or update version to tag it
  correctly; haven't pulled that yet.
- **Widescreen override**, once/if solved per item 2, ships as code
  in `overrides/` instead of a cheat (it has to run every frame and
  touch floating point, which the AR cheat engine doesn't do).
