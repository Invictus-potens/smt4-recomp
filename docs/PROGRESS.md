# Progress

## Title

*Shin Megami Tensei IV* (USA), title ID `00040000000E5C00`.

## Build status (2026-10-13)

- `3dsrecomp analyze`: 95.1% of code found, 12,269 functions, 1.5%
  unreached.
- First `3dsrecomp build` (library mode, no overrides): wrote 194 MiB
  of C across 170 chunks, ~25 minutes to build on a Ryzen/RTX 2060
  machine. Loaded fine in Zakuro, 114,697 entry points.
- Boots to the title screen and in-game menus correctly over Vulkan.
  An earlier "blank screen" result was a screen-capture artifact in
  the tooling used to check it (Windows Graphics Capture not reading
  the Vulkan mailbox-mode swapchain), not an emulation bug. Confirmed
  by capturing a full desktop composite instead of the window alone.
- Cheats: community Action Replay / Gateway codes for this title
  (`cheats/00040000000E5C00.txt`) load and toggle correctly in
  Zakuro's built-in cheat UI. The community addresses for Macca
  (`0x08291C8C`), hero EXP (`0x08291CC0`) and App Points (`0x0829B470`)
  were confirmed in game.

## Enhancements (2026-10-14)

- **Multipliers** for Macca, hero EXP, demon EXP and App Points, x2/x3/x5,
  switched on and off from Zakuro's cheat menu while playing. All four
  tested in game. See [MULTIPLIERS.md](MULTIPLIERS.md).
- **60 FPS** code by lucky, in the cheats file, and in Zakuro's built-in
  enhancements ([fearkov/zakuro#16](https://github.com/fearkov/zakuro/pull/16),
  merged). It makes the whole game run twice as fast, and turning it off
  doesn't bring the normal speed back: the code's 0 stays at `0x004E3BA0`
  (the game's value is 1), and a state saved meanwhile keeps it. The
  `30 FPS (normal speed)` cheat writes the 1 back. Reported upstream as
  [fearkov/zakuro#17](https://github.com/fearkov/zakuro/issues/17), with
  the same 30 FPS code added to the enhancements in
  [fearkov/zakuro#18](https://github.com/fearkov/zakuro/pull/18) (merged).
- **Widescreen** 16:9 and 21:9 (2026-10-15): the game's projection
  widened by an override, picked from the cheat menu, with Zakuro's new
  `Top screen shape` setting stretching the top screen to match. Played
  in town, dungeon, battles and dialogue at 16:9 with nothing wrong. The
  Zakuro half was merged upstream in
  [fearkov/zakuro#19](https://github.com/fearkov/zakuro/pull/19).
  See [WIDESCREEN.md](WIDESCREEN.md).
- Planned, not started: quality-of-life cheats (no random encounters,
  full compendium, easy recruit) and others in
  [ENHANCEMENTS.md](ENHANCEMENTS.md).

## Known issues / open questions

- During boot, the game calls `srv:GetServiceHandle("APT:U")`
  several hundred times in quick succession (observed ~650 times in
  ~30s before reaching the title screen). Zakuro's `GetServiceHandle`
  allocates a fresh kernel handle each call, by design (matches real
  3DS `srv` semantics for session handles). Whether the game is
  properly closing these handles, or this is a leak that would
  eventually exhaust handle space in a long session, is unconfirmed.
  Not reported upstream: not enough evidence to be worth filing.
- **Fatal crash after naming the character: FIXED upstream in Zakuro
  v0.2.27** ([fearkov/zakuro#15](https://github.com/fearkov/zakuro/issues/15)).
  Root cause: `GetSemaphoreEventHandle` cached the kernel event handle
  it created and never reset that cache in `UNLOAD_COMPONENT`. The
  title closes its copy of that handle as part of shutting the DSP
  down for a scene transition (the usual `ndspExit`/`ndspInit`
  pattern), then asks for the handle again when it reloads the DSP
  for the next scene, and Zakuro handed back the same, now-closed handle
  number instead of a fresh one, and the title's own `svcSignalEvent`
  on it failed as invalid (`0xD8E007F7`, kernel/invalid-handle,
  result desc 1015), reported as a fatal error at pc `0x0013A8D0`.
  Traced with a short debug-logged repro (loaded a save right before
  the crash point) that caught the exact handle lifecycle: DSP shut
  down → `thread5`/`thread6` exit → DSP firmware reloaded → crash.
  fearkov fixed it independently in v0.2.27 (`GetSemaphoreEventHandle`
  now hands out a fresh handle to the same event every time instead of
  caching one), landing within hours of the issue being filed.
  Confirmed the title continues past the crash point and into
  real gameplay with `smt4-recomp` rebuilt against v0.2.27.
- Running from a save state, Zakuro's interpreter takes over for a while:
  threads come back mid-function, where the linked code has no entry, so
  hundreds of distinct spots run interpreted until execution reaches a
  known entry. Harmless, but code hooked by an override doesn't run its
  override until then.

## Next steps

- Watch handle counts over a longer play session to settle the
  `APT:U` question above.
- Keep playing to find the next issue, if any.
- Upstream, waiting on the maintainer: issue
  [#17](https://github.com/fearkov/zakuro/issues/17) (cheats left in place
  when turned off). #18 and #19 are merged.
- Quality-of-life cheats: no random encounters, full compendium, easy
  recruit.
