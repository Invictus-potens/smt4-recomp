# Resource multipliers

Macca, hero EXP, demon EXP and App Points multipliers (x2/x3/x5), picked from Zakuro's
cheat menu while the game runs. Code: `overrides/multipliers.c` and
`scripts/patch_generated.py`. Cheats: `cheats/00040000000E5C00.txt`.

| Resource | Cheats | Control byte | Hooked at |
|---|---|---|---|
| Macca | `Macca x2/x3/x5` | `0x004EFF00` | `RECOMP_OVERRIDE(0x00295EAC)` and `(0x00295EA8)`, addMacca |
| Hero EXP | `Hero EXP x2/x3/x5` | `0x004EFF01` | inline add at `0x0018CF84`, patched in the generated C |
| Demon EXP | `Demon EXP x2/x3/x5` | `0x004EFF03` | inline load at `0x0018D190`, patched in the generated C |
| App Points | `App Points x2/x3/x5` | `0x004EFF02` | `RECOMP_OVERRIDE(0x0026E8A0)`, addAppPoints |

Only gains are multiplied; spending is untouched. Turn on one multiplier
per resource; with two on, whichever comes later in the file wins.

Shop and battle-result screens show the gain they computed themselves, not
the multiplied one. The saved total is multiplied: reopen the shop or the
status screen to see it.

## How the cheat menu drives an override

Zakuro's cheats can only write memory, and an override can only read it,
so they meet at a control byte:

- each `[... xN]` cheat writes `N` to its control byte every frame;
- the override reads the byte when the game adds that resource, uses it
  as the multiplier, and **clears it**, keeping the value for 100 ms so
  several gains in the same frame all get it;
- turn the cheat off and nothing writes the byte again, so it reads 0 and
  the multiplier drops back to x1. Zakuro doesn't undo a cheat's writes
  when it's turned off, which is why the override has to clear the byte.

The control bytes sit in the slack at the end of `.bss`: the exheader puts
`.data` at `0x0042B000` (`0x4B644` bytes) and `.bss` right after it
(`0x797F0` bytes), so the game's data ends at `0x004EFE34`, but the
loader maps whole pages, up to `0x004F0000`. Those 460 bytes are mapped,
zeroed, and nothing in the game addresses them.

## How each hook was found

### Watchpoint

A probe build (`3dsrecomp`'s `recomp.h` patched in a throwaway copy of
the project) logged every guest write to the save-data addresses the
community cheats use (Macca `0x08291C8C`, hero EXP `0x08291CC0`, App
Points `0x0829B470`), with the block address that wrote it:

- `BLOCK_TO` was changed to store its address in `ctx->r[15]`, so a
  write knows roughly where it came from (to the basic block);
- the check had to go inside `mem_write8/16/32_in`, because their fast
  path writes the page directly with `memcpy` and never reaches the
  host's `write32` callback.

Writes only show *where the total is stored*, not who computes the gain,
and a few of them are misleading (below), so each hook was confirmed by
reading the generated C around it and by a test in game.

### Macca: `0x00295EAC`

```
00295EAC  push {r4-r6,lr}
00295EB0  mov  r5, r0             ; r5 = delta (r0, the only argument)
00295EB4  bl   0x002A8A08         ; r0 = &macca
00295EB8  mov  r4, r0
00295EBC  ldr  r0, [r0]           ; r0 = macca
00295EC4  add  r1, r0, r5
00295EC8  str  r1, [r4]           ; macca += delta
00295ECC  ble  0x00295EDC         ; delta <= 0: skip the tracking call
00295ED8  bl   0x0029C0FC         ; (0, delta): macca earned
00295EDC  ...                     ; clamp to [0, max]
```

`0x002A8A08` takes no argument: `ldr r0,=global; ldr r0,[r0,#4];
add r0,r0,#0x8C; bx lr`, so `base = *(global+4) = 0x08291C00` and Macca is
`base+0x8C`. `0x00295EA8` is a `mov r0,r0` right before the function that
a few callers enter at instead.

Two traps on the way:

- the battle-results function calls addMacca with the battle's Macca
  reward, which **is 0 in many early fights**. Before a shop sale was
  tried, the hook looked broken because every call had delta 0;
- `f_0026FA94` writes `0x08291C8C` too, but it stores 0: it's
  initialization, not a gain.

### App Points: `0x0026E8A0`

App Points (`0x0829B470`) are `base+0x9870`. No literal pool holds that
offset; it is built with two immediates. Searching `.code` for
`ADD Rd, Rn, #0x9800` gives five sites. Two are the initialization
routines the watchpoint caught, two are getters for neighbouring fields
(`+0x981E`, `+0x9874`), and `0x0026E8A0` is the adder:

```
0026E8A8  ldr  r2, [r1,#4]        ; base
0026E8AC  add  r1, r2, #0x9800
0026E8B0  add  r1, r1, #0x6C      ; base+0x986C: App Points earned, lifetime
0026E8B4  add  r2, r2, #0x9000
0026E8BC  add  r2, r2, #0x870     ; base+0x9870: App Points now
0026E8C0  add  ip, ip, r0         ; both += delta (r0)
...                               ; clamp the current value to the max
```

### Hero EXP: inline in the battle results, `0x0018CF84`

There's no add-EXP function to override. The battle-results function
(`f_0018BFEC`, ~18k lines of C) adds it inline:

```
0018CF70  bl   0x002B66EC         ; r0 = &hero
0018CF74  mov  r5, r0
0018CF78  ldr  r0, =0x35B4        ; offset into the battle results
0018CF7C  ldr  r1, [r5,#0x30]     ; hero.exp
0018CF80  ldr  r0, [r0, r4]       ; gain = results[0x35B4] (r4 = results)
0018CF84  add  r7, r1, r0
0018CF88  str  r7, [r5,#0x30]     ; hero.exp += gain
```

`r7` is used again for the level-up check right after, so multiplying the
gain at the add covers that too. `scripts/patch_generated.py` rewrites the
add's operand to `smt4_boost_exp(ctx, reg0_)`, anchored on the instruction
comment so it fails instead of patching the wrong spot after a new port.

The literals in this function (`0x35B4` EXP, `0x3650` Macca, `0xD558`)
are **offsets** into the battle-results structure, not addresses.

## Tools

- `codetool.py` (kept outside the repo, it reads the ROM): extracts and
  BLZ-decompresses `.code` from the ROM, reads literal pools, and finds
  which functions reference a value.
- The watchpoint/probe build lives in a throwaway copy of the project.
  The Zakuro-side diagnostics (an interpreter-entry log for linked builds,
  and a guest-write watch in `Memory`) are on a local `diag-watch` branch
  of the Zakuro fork, not published.

### Demon EXP: inline in the battle results, `0x0018D190`

Further down the same function, a loop over the 24 stock slots gives each
demon its EXP. Demons use the same member structure as the hero (EXP at
`+0x30`), and the gains sit right after the hero's in the results:

```
0018D15C  ldrsb r1, [r9,#0xF0]    ; demon id of this slot, < 0 = empty
0018D16C  bl   0x002A89E4         ; r6 = &demon
0018D178  ldr  r1, [r6,#0x30]     ; demon.exp
0018D180  add  r0, r4, r0, lsl #2
0018D184  add  r0, r0, #0x3400
0018D188  add  r0, r0, #0x1B8     ; &results[0x35B8 + id*4]
0018D18C  str  r0, [sp,#0x40]     ; kept for later
0018D190  ldr  r0, [r0]           ; gain
0018D194  add  r0, r0, r1
0018D19C  str  r0, [r6,#0x30]     ; demon.exp += gain
```

In some cases the function undoes the add at `0x0018D230`, rereading the
gain through the pointer saved at `[sp,#0x40]` (`exp - gain`). Multiplying
only at the add would leave the extra EXP behind after an undo, so the
patch boosts the gain where it's loaded and **writes it back**: the add,
the undo and anything after them all see the same value.

Party demons and stock demons both go through this loop (stock demons
get a smaller share), so `Demon EXP xN` covers all of them.
