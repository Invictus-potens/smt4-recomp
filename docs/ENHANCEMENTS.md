# Enhancements roadmap

Ideas for improving the SMT4 experience on top of the stock recomp,
roughly ordered by effort.

## 1. Split/separate screens: already done, zero code

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
are the two worth using for SMT4. The bottom screen is only a
menu/status display outside of a few touch-specific UI bits (naming,
the Gauntlet Hunt log), so losing it costs little.

No work needed here beyond telling people the option exists.

## 2. True widescreen: working, waiting on Zakuro

Done as a prototype: the game's projection is patched to draw a wider
scene, and a new Zakuro setting stretches the top screen to match. See
[WIDESCREEN.md](WIDESCREEN.md). The Zakuro half was merged in
[fearkov/zakuro#19](https://github.com/fearkov/zakuro/pull/19).

The notes below are from before it was done.

What "Top screen only" does NOT do: it doesn't change the rendered
aspect ratio. The 3D scene is still composed for the 3DS's 400×240
(5:3) top screen and just gets letterboxed/pillarboxed to fit a 16:9
window. Actual widescreen needs the game's own camera/projection code
patched to render more horizontally, which is exactly what TriAevum did for
Ocarina of Time 3D with a custom renderer, and what the N64/GameCube
recomp scene does by rewriting the camera entirely.

Searched for an existing Action Replay / Gateway widescreen code for
SMT4 specifically and found none. Widescreen AR codes are rare for 3DS
titles in general (unlike GBA/DS where screen-stretch hacks are
common), because the projection matrix is usually computed from
runtime values in a way a memory-write-only cheat code can't patch
reliably across every frame.

Realistic paths, hardest to easiest:
- **Find and override the camera/projection function.** Use the
  generated C + a disassembler (IDA/Ghidra) to find where the
  perspective matrix is built (look for a function writing to the
  PICA200 `PERSPECTIVE_PROJ` floats; it'll take an aspect ratio
  constant, probably `4:3`-ish, 400.0/240.0). Write a
  `RECOMP_OVERRIDE` (see `docs/overrides.md` in 3dsrecomp) that
  patches the aspect ratio before calling through to the original.
  This is the only way to get genuinely more horizontal view, not
  just a stretched image. Needs real reverse-engineering time.
- **Cheap stretch, not true widescreen.** A cheat or override that
  scales the final framebuffer non-uniformly (stretches the existing
  4:3 render to 16:9) distorts character models, which looks worse
  than letterboxing for a game like this, not worth doing.
- **Ask upstream.** Worth a Discord question to fearkov/the Zakuro
  community. This could become a general Zakuro feature (home-grown
  FOV widening per title, similar to Citra/Azahar's old "Stretch to
  Window" vs actual 3D hacks some communities maintain per game) that
  benefits every title, not just SMT4.

Given the effort, this is a "maybe later" item, not a quick win.

## 3. More / different cheats

Added so far (`cheats/00040000000E5C00.txt`, Citra/Gateway AR format
which Zakuro reads natively):

- Max Macca, 9999 App Points, All Items
- Hero Max Exp, HP/MP, Stats
- **60 FPS** and **30 FPS** (by lucky): were cheats here, now built into Zakuro as enhancements
- **Macca / Hero EXP / Demon EXP / App Points x2, x3, x5**: these switch overrides in this
  build on and off rather than writing game values. Done, see
  [MULTIPLIERS.md](MULTIPLIERS.md).

Planned quality-of-life cheats. For each one, first look for an existing
community code (JourneyOver's repo, GBAtemp's SMT4 cheat thread,
gamegenie.53lu.com); if there isn't one, find the game's own check and
switch it with an override and a control byte, the way the multipliers
work:

- **No random encounters**: stop the encounter check from triggering
  while walking in the field and dungeons. Story and boss fights stay.
  Best as a toggle, so grinding is still possible with it off.
- **Full compendium**: register every demon in the compendium so any of
  them can be summoned (for Macca). The GBAtemp cheat-table thread
  (LuckyCross) has per-demon codes for Cheat Engine that could be ported
  to the Gateway/AR format Zakuro reads, but that's dozens of codes;
  finding the registration flags or the "add to compendium" function
  and filling it in one go is likely simpler.
- **Easy recruit**: make negotiations succeed, for example by having the
  demon accept at the first answer or skipping the item and Macca
  demands. Needs the negotiation outcome check found in the code.

Other ideas:
- **Instant kill**: battle cheat, lower priority.
- **60 FPS as a Zakuro "enhancement"**: merged as
  [fearkov/zakuro#16](https://github.com/fearkov/zakuro/pull/16) (rev0). A
  30 FPS one to undo it was merged in
  [fearkov/zakuro#18](https://github.com/fearkov/zakuro/pull/18).
- **Widescreen override**: done, in `overrides/widescreen.c` (item 2).
