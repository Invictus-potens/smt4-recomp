# Assets

The recompilation only touches the game's code (the ExeFS `.code`, about
4 MB of ARM). Everything else, models, textures, sound, video and text, is in
the game's RomFS, which the program reads from your ROM while you play, the
same way the console does. So the assets are extracted from the ROM directly,
recompiled or not.

`tools/extract` does that and converts what it can to common formats. Like the
build, it only works with your own decrypted dump, and **what it writes is
the game's: don't share it**.

## Using it

```
cd tools/extract
cargo build --release
target\release\smt4-extract "C:\path\to\Shin Megami Tensei IV (USA).3ds" C:\smt4-assets
```

It needs only Rust (no C compiler, no `code/`). `--only` picks what to do,
from `raw,textures,models,text,audio,video` (all of them by default):

```
smt4-extract <rom> <out> --only textures,text
```

Audio needs [vgmstream](https://github.com/vgmstream/vgmstream/releases)'s
`vgmstream-cli` on `PATH`, and video needs [FFmpeg](https://ffmpeg.org)'s
`ffmpeg`; without them those steps are skipped, and the files are still in
`romfs/` as they are. Textures, models and text take under a minute; audio
and video a few minutes.

## What comes out

| Folder | From | Count | Format | Size |
|---|---|---|---|---|
| `romfs/` | every file in the RomFS | 20,126 | as they are | 1.8 GB |
| `textures/` | `.stex`, `.cmp` | 3,421 | PNG, plus `pack-names.txt` | 210 MB |
| `models/` | `.bam`, `.bcmdl` | 850 | CGFX (`.bcres`) | 480 MB |
| `text/` | `.mbm` | 1,170 | UTF-8 text | 5 MB |
| `audio/` | `.bcstm` (music), `.bcwav` (sounds, voices) | 368 + 8,038 | WAV | 3.7 GB |
| `video/` | `.moflex` | 26 | MP4 (H.264, 400x240) | 145 MB |

All of it takes about 7 GB; `--only` skips what you don't need.

Each converted file keeps its RomFS path, so
`tex/stex/devbu/devbu010.stex` becomes `textures/tex/stex/devbu/devbu010.png`.

## The formats

### Textures: `.stex`, `.cmp`

An Atlus header over a texture in the PICA200's own tiled layout:

| Offset | | |
|---|---|---|
| 0x00 | `STEX` | magic |
| 0x08 | `0x0DE1` | `GL_TEXTURE_2D` |
| 0x0C, 0x10 | width, height | |
| 0x14 | GL type | `0x1401` unsigned byte, `0x8363` 5-6-5, `0x8034` 5-5-5-1, `0x8033` 4-4-4-4, `0x6760` 4-4, `0x6761` 4 bits |
| 0x18 | GL format | `0x6752` RGBA, `0x6754` RGB, `0x6756` alpha, `0x6757` luminance, `0x6758` luminance-alpha, `0x675A` ETC1, `0x675B` ETC1A4 |
| 0x1C | data size | sometimes larger than the data, so the extractor works the size out from the format |
| 0x20 | data offset | always `0x80` |

The pair of GL constants (the CTR SDK's `*_NATIVE_DMP` names) gives the
PICA200 format, and the pixels are decoded with Zakuro's own texture decoder,
the one it renders the game with. A `.cmp` is an `.stex` compressed with
LZ11. Most are ETC1A4 (demon portraits) and ETC1 (backgrounds).

### Models: `.bam`, `.bcmdl`

`.bcmdl` files are CGFX, Nintendo's standard 3D format for the 3DS (also
called BCRES): 850 files holding about 1,000 models, 5,000 textures and 400
skeletal animations. `.bam` files are the same CGFX behind a 32-byte Atlus `ATBC`
header; a couple are only the header, with no model. Each holds the meshes,
skeleton, materials, textures and often animations of a demon, character,
map or effect.

The extractor cuts the CGFX out as `.bcres`. To view or convert one to
DAE/OBJ/glTF, open it with a CGFX tool such as
[SPICA](https://github.com/gdkchan/SPICA), Ohana3DS or Noesis.

### Text: `.mbm`

`MSG2` files: a 0x20-byte header (count at 0x10, table offset at 0x14), a
table of `(id, size, offset, 0)` entries, then the messages: Shift-JIS, with
Latin text in full-width letters, control codes starting with `F8`, and
`FF FF` at the end. The extractor writes each as `[id]` and the text, with the
codes as:

| Code | Arguments | Written as |
|---|---|---|
| `F8 01` | | a new line |
| `F8 02` | | `{next}`, waits for a button before the next box |
| `F8 12` | the name, then `00 00` | `{speaker Name}` |
| `F8 13` | u16, u16, u32 voice id | `{voice id}`, the voice clip |
| `F8 7D` | u16 | `{7D n}`, a value the game fills in (item target, limit, ...) |
| `F8 7B` | 4 × u16 | `{7B a b c d}` |
| `F8 14`, `F8 7C` | 2 × u16 | `{14 a b}` |
| others | u16 | `{XX n}` |

The game also carries a few files in Japanese (some battle events, old map
and quest text) that use furigana markup (`[kanji/reading]`), and 16 of them
use a different set of control codes the extractor doesn't read yet.

### Sound: `.bcstm`, `.bcwav`

Nintendo's standard stream (music) and wave (sound effects, voices) formats,
DSP-ADPCM. vgmstream converts both. Voice clips are numbered by the `{voice}`
ids in the text.

### Video: `.moflex`

Mobiclip video, used for the animated cutscenes. They're all stereoscopic
3D (named `(3d)`); FFmpeg reads them natively and the MP4 has one eye.

### Not converted

| Files | What | |
|---|---|---|
| `.bf` | Event and battle scripts | `FLW0`, Atlus's flowscript bytecode. [Atlus Script Tools](https://github.com/tge-was-taken/Atlus-Script-Tools) decompiles it for other Atlus games; not tried on SMT4. |
| `.tbb`, some `.bin` | Data tables (items, quests, demons) | `TBCR` tables; no known layout yet. |
| `.col` | Map collision | `MAP4`. |
| `.eft`, `.epl` | Effects | |
| `.tga` | Small textures | Plain TGA, already readable. |
| `.earg`, `.ttd`, `.bcenv`, `.bccam`, `.bcanim` | Effect arguments, camera, lights, animations | `.bc*` are CGFX parts. |

A `.cmp` texture is written as `<name>.cmp.png`, as a few sit next to an
`.stex` of the same name.

`textures/pack-names.txt` lists, for each extracted PNG, the name a texture
pack picture needs to take its place in Zakuro (or Citra, Azahar):
`tex1_<width>x<height>_<hash>_<format>_mip0.png`, the hash taken with
Zakuro's own code over the texture as the game hands it to the GPU.

## Putting assets back: mods

Replacing images with texture packs, and `cargo fit-texture` (`tools/fit-texture`), which
makes and installs a picture for one: [MODDING.md](MODDING.md).

Zakuro also takes files that replace the game's own at their RomFS paths, in
`%APPDATA%\zakuro\mods\00040000000E5C00\romfs\` (on macOS, under
`~/Library/Application Support/zakuro`; a translated `.mbm`, a new
model), but writing files back in the game's formats (text back to `.mbm`,
a PNG back to `.stex`) isn't done yet.
