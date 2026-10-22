# Easy Recruit

The `[Easy Recruit]` cheat (control byte `0x004EFF05`) makes every demon
negotiation go the player's way: answers please the demon, refusing or not
having what it asks for still works, and it doesn't run off, take a gift and
leave, or cut the talk short. The demon still asks for things. Turning the
cheat off goes back to normal negotiation on its own.

Code: `overrides/recruit.c` and one patch in `tools/build/src/patch.rs`.

## How negotiation works

Negotiation isn't in the game's compiled code: it's a flowscript,
`battle/talk/talk.bf` (Atlus's `FLW0` bytecode). Its procedure names were
kept, which makes it readable once decoded: `talkLoop_ChkSuccess` (does an
answer work), `reqLoop_ChkRejectSuc`, `reqLoop_ChkAcceptFail`,
`reqLoop_ChkShortSuc`, `reqLoop_ChkCheat`, `reqLoop_ChkCut`,
`reqLoop_ChkRunAway`, `robLoop_chkAngry` and so on.

Each check works out a chance and rolls it with script native `0x013F`. But
first, each one asks a debug setting through native `0x0259` and, if it's
set, returns the forced result instead. In the shipped game those settings
are all 0. They're the `TalkCtrl` entries of the game's leftover debug menu,
in the same order (FTALK is index 0):

| Index | Setting | Forced to | Effect |
|---|---|---|---|
| 7 | ANGRY | 0 | the demon isn't angry from the start |
| 12 | TALKLOOP_SUC_CTRL | 1 | every answer works (2 would make it fail) |
| 15 | REQLOOP_CHEAT_CTRL | 2 | it doesn't take the gift and run |
| 16 | REQLOOP_CUT_CTRL | 2 | it doesn't cut the talk short |
| 20 | REQLOOP_REJECT_SUC | 1 | refusing a demand still goes well |
| 21 | REQLOOP_ACCEPT_FAIL | 0 | giving what it asks doesn't fail |
| 22 | REQLOOP_SHORT_SUC | 1 | not having what it asks still goes well |
| 23 | REQLOOP_RUNAWAY | 0 | it doesn't run away |
| 24 | ROBLOOP_NOT_ANGER | 1 | it doesn't get angry at a robbery |

## The hook

The script's natives are a table at `0x00406038`, 8 bytes each (function,
argument count), indexed by the native's number; the interpreter calls them
from `f_001A07CC`. Native `0x0259` is `f_002F127C`:

```
002F1280  bl   0x293c98        ; the talk context, into r4
002F128C  bl   0x296f60        ; the script's argument 0, the index
002F1290  add  r0, r4, r0, lsl #2
002F1294  add  r0, r0, #0x2000
002F1298  ldr  r0, [r0, #0x2a8] ; context + 0x22A8 + index * 4
002F129C  bl   0x28df54        ; the script's return value
```

The patch passes the load at `0x002F1298` through `smt4_talk_debug(ctx,
index, value)`, with the index worked out from the address and r4. While the
cheat is on, it returns the values in the table above; otherwise, the game's
own.

## Tools

The ones used here are kept outside the repository, in the author's
working folder: a FLW0 disassembler (procedures, labels, the decoded
instructions, `COMM` with the native's number) and a tracing build of the
game that logs every function the first time it runs from each caller, to
find what an action in the game runs.
