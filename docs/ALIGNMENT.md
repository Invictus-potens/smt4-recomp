# Alignment

Shin Megami Tensei IV keeps the hero's Law/Chaos alignment hidden; it decides
which ending you can get. Two cheats show it:

| Cheat | What it shows |
|---|---|
| **Show Alignment** | "Alignment: Law (+12)" in the bottom right corner, all the time |
| **Alignment Notices** | "Alignment +5 toward Law: Neutral (+8)" at the top for a moment, each time a choice changes it |

They can be used together or on their own. Turning a cheat off hides it
again. The sides follow the endings: **+9 and up is Law, -9 and down is
Chaos, Neutral between.**

Code: `src/alignment.rs`, shown through Zakuro's `GameOverlay` (see below).

## Where the game keeps it

Choices in event scripts (`event/*/*.bf`, Atlus flowscripts) change the
alignment through script native `0x007D`. It's called with one constant, in
pairs of branches right after a choice: `-1`/`+1`, `-5`/`+5`, `+10` and so on,
the amounts the guides list. In 117 calls across 55 scripts, those are the
only values it takes.

The native is `f_002F5FE4` (the script natives are a table at `0x00406038`,
8 bytes each). It reads its argument as a signed halfword and calls
`f_00334064(delta)`:

```
00334064  push   {r4, lr}
00334068  mov    r4, r0              ; delta
0033406C  ldr    r0, =0x1FD          ; three game flags: while one of them
00334070  bl     0x2c0ba8            ; is set, the alignment no longer
...                                  ; changes (0x1FD, 0x1FE, 0x1FF)
0033409C  bl     0x2b66ec            ; the save data
003340A4  ldrsh  r0, [r0, #0x74]     ; alignment
003340A8  add    r0, r0, r4
003340AC  cmn    r0, #0x3e8          ; clamped to -999..999
...
003340C0  strh   r0, [r1, #0x74]
```

`f_002B66EC` returns `[0x0043E2A8 + 4] + 0x90`, so the alignment is the
signed halfword at **`[0x0043E2A8 + 4] + 0x104`**. The pointer is 0 until the
game has made its state; the overlay shows nothing until then.

## How it's shown

Zakuro has no way, by itself, for a game's own program to draw over the
game. This project adds one to Zakuro: `zakuro::run_with(linked, overlay)`
takes a `GameOverlay`, which gets the game's memory after every frame and
returns a line for the corner and, now and then, a notice. Zakuro draws them
with its own menus, so nothing in the game's screens or code is touched.
It's proposed upstream; until it's there, the build uses the branch on the
fork.

`src/alignment.rs`:
- reads each cheat's control byte (`0x004EFF06` Show Alignment,
  `0x004EFF07` Alignment Notices), clearing it after reading, as the
  multipliers do; a cheat counts as on for 200 ms after its byte was last
  seen, so turning it off hides the text;
- reads the alignment and shows it with its side;
- shows a notice when it changes by up to 30, the most one choice changes
  it; a bigger jump is a save loaded or a new game, taken without a notice.

## Ideas for later

- **In the game's own status screen** (option C when this was planned):
  the value drawn where the hero's stats are, as if the game showed it. It
  takes changing the code that draws that screen and its text or textures,
  much more work than the overlay.
