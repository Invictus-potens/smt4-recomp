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
  the Vulkan mailbox-mode swapchain), not an emulation bug — confirmed
  by capturing a full desktop composite instead of the window alone.
- Cheats: community Action Replay / Gateway codes for this title
  (`cheats/00040000000E5C00.txt`) load and toggle correctly in
  Zakuro's built-in cheat UI. Not yet verified to write correct values
  in an active save (needs a loaded game, not just the title screen).

## Known issues / open questions

- During boot, the game calls `srv:GetServiceHandle("APT:U")`
  several hundred times in quick succession (observed ~650 times in
  ~30s before reaching the title screen). Zakuro's `GetServiceHandle`
  allocates a fresh kernel handle each call, by design (matches real
  3DS `srv` semantics for session handles). Whether the game is
  properly closing these handles, or this is a leak that would
  eventually exhaust handle space in a long session, is unconfirmed.
  Not reported upstream — not enough evidence to be worth filing.
- **Fatal crash after naming the character — FIXED upstream in Zakuro
  v0.2.27** ([fearkov/zakuro#15](https://github.com/fearkov/zakuro/issues/15)).
  Root cause: `GetSemaphoreEventHandle` cached the kernel event handle
  it created and never reset that cache in `UNLOAD_COMPONENT`. The
  title closes its copy of that handle as part of shutting the DSP
  down for a scene transition (the usual `ndspExit`/`ndspInit`
  pattern), then asks for the handle again when it reloads the DSP
  for the next scene — Zakuro handed back the same, now-closed handle
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
- Overrides directory is empty so far; nothing has needed a hand
  fix yet.

## Next steps

- Load an actual save or start a new game to confirm the cheats write
  the expected memory values in practice.
- Re-run `3dsrecomp build`/`port` with `--overrides overrides/` once
  any fixes are written, to confirm the override mechanism works for
  this title.
- Watch handle counts over a longer play session to settle the
  `APT:U` question above.
- Keep playing past where #15 used to block progress to find the next
  issue, if any.
