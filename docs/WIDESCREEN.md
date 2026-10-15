# Widescreen

The 3D scene drawn wider than the 3DS's 5:3 top screen: more view at the
sides, the same view top to bottom (Hor+), nothing stretched.

It takes two parts that have to agree:

1. **The game draws a wider scene.** A `Widescreen 16:9` or
   `Widescreen 21:9` cheat (control byte `0x004EFF04`) makes the game's
   projection use that aspect instead of 400/240. Code:
   `overrides/widescreen.c` and two patches in
   `scripts/patch_generated.py`.
2. **Zakuro shows the top screen that wide.** The game still draws into
   the console's 400x240 framebuffer, so the wider scene comes out squeezed
   until it's stretched back out. That's the `Top screen shape` setting
   (Settings, Graphics), in Zakuro since
   [fearkov/zakuro#19](https://github.com/fearkov/zakuro/pull/19).

On its own, the cheat squeezes the picture and the setting stretches it;
together they cancel out into a correct, wider view. Use the same shape in
both, and the `Top screen only` layout.

On a 16:9 monitor the gain is about 7% more width, since the 3DS top screen
is already 5:3; on 21:9 it's about 40%. 16:10 isn't offered: it's
narrower than 5:3.

## Where the projection comes from

Searching `.code` for the float 400/240 (`0x3FD55555`) and the first call
after each load of it, 13 of them go to `f_002C1FFC`, the camera's
`SetPerspective`:

```
002C1FFC  push {r4-r6,lr}          ; r0 = camera, s0 = fovy in degrees,
002C2018  vstmia r0, {s0, s1}      ; s1 = near, s2 = far, s3 = aspect,
002C2024  vstmia r0, {s2, s3}      ; kept at camera+0x28..0x34
002C2020  vmul.f32 s0, s0, s16     ; degrees to radians
002C2038  bl  0x002B761C           ; perspective(camera+0xA8, fovy, aspect, near, far)
002C2044  bl  0x002BC0B4           ; rotate for the 3DS's sideways framebuffer
002C206C  bl  0x002B761C           ; again into camera+0xE8, unrotated
```

`f_002B761C(mtx, fovy, aspect, near, far)` is a standard perspective
matrix: it gets sin and cos of fovy/2 from the game's table
(`f_002AD5A0`), then `m[1][1] = cot(fovy/2)` and
`m[0][0] = cot(fovy/2) / aspect`. Two more functions (`f_0019F99C`,
`f_001DB28C`) call it directly, also with 400/240.

The patches pass the aspect through `smt4_widen_aspect` on entry to both
functions: `s3` for the camera, so the aspect it keeps (and anything that
reads it later, like visibility checks) is the wide one too, and `s1` for
the matrix, for the direct callers. Only exactly 400/240 is changed, so the
camera's call into the matrix isn't widened twice, and other projections
(different aspects) are left alone.

The game sets its camera when a scene loads, so turning the cheat on or off
shows at the next scene change.

## The Zakuro side

`zakuro_gpu::backend::layout` places the screens from a logical size of
400x240 for the top one. The `Top screen shape` setting makes that width
wider (`240 * 16/9`, or `560` for 21:9) in every layout, and the window size
follows. The presenters already stretch each screen's image into its
rectangle, so nothing else changes.

It was merged upstream in
[fearkov/zakuro#19](https://github.com/fearkov/zakuro/pull/19), as a setting
any game with a widescreen code can use. It isn't in a Zakuro release yet
(the latest is v0.2.27), so this project builds against Zakuro's git main.

## Known limits

- 2D elements drawn with their own projection (HUD, menus) are stretched
  along with the picture. At 16:9 it's 7% and hard to see; at 21:9 it shows.
- Played at 16:9 through town, a dungeon, battles and dialogue scenes:
  no pop-in at the edges, nothing broken.
- Anything the game culls with its own view checks that don't use the
  camera's aspect could pop in at the new edges. Not seen yet.
