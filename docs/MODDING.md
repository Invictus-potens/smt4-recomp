# Modding: replacing the game's images

Any 2D image the game shows (portraits, icons, menus, backgrounds) can be
replaced with your own, at the same resolution or higher, without touching the
ROM or rebuilding anything. This uses Zakuro's **texture packs**, the same
format Citra and Azahar use: a folder of PNGs, each named after a hash of the
texture it replaces. While drawing, Zakuro swaps the game's texture for your
picture.

Everything here works with the plain program from `cargo build-game`. You need:

- the textures extracted with `tools/extract` (see [ASSETS.md](ASSETS.md)),
  which also writes `textures/pack-names.txt`, the pack name of every texture;
- **Draw the 3D on the GPU** on in Settings (it is by default): texture packs
  only apply to what the GPU draws.

## Replacing an image, step by step

1. **Find it** among the extracted PNGs, in `<assets>\textures\tex\stex\...`.
   The folders say roughly what's inside:

   | Folder | What |
   |---|---|
   | `stbustup/` | full-body portraits, status screen and battle (`st_bu_pc001` is the hero, `st_bu_en<id>` demons) |
   | `devbu/`, `DevilGtix/` | demon portraits and smaller demon pictures |
   | `BUSTUPCHAR/`, `bustup/` | character portraits for events and dialogue, one per expression |
   | `faceiconall/`, `charicon/` | small face icons (party bar, lists) |
   | `status/` | status screen parts (`st_dot_pc001`, the hero's small sprite) |
   | `bg/`, `commonbg/`, `BUSTUPBG/` | backgrounds |
   | `commandmenu/`, `messagewindow/`, `buttonhelp/`, `NUMBERFONT/` | menus, message boxes, button prompts, digits |
   | `automap/`, `mapnavi/`, `icon/` | map and icons |

2. **Make the picture** and install it in one go:

   ```
   cargo fit-texture <assets>\textures\tex\stex\stbustup\st_bu_pc001.png my_picture.png
   ```

   It works out the name Zakuro looks for (from `pack-names.txt`), fits your
   picture in, and saves it in
   `%APPDATA%\zakuro\mods\00040000000E5C00\textures\` (on macOS,
   `~/Library/Application Support/zakuro/mods/00040000000E5C00/textures/`).

3. **Restart the game.** Zakuro reads the pack when the game starts; its log
   says `texture pack in ..., N pictures`.

To undo, delete the picture from that folder (or the whole folder).

## How the picture is fitted

Most portraits are a character inside a mostly transparent texture: the
hero's `st_bu_pc001` is 256x256, but he only takes up 181x236 of it, standing
on the bottom edge. The game places and scales the whole texture, so a picture
that fills all of it comes out much bigger than the character did (that's what
happened in battle the first time).

So `cargo fit-texture` measures the area the original's visible pixels take up,
and fits your picture inside that area, keeping its proportions, centered
across and resting on the bottom, leaving the rest transparent. Options:

- `--fill` covers the whole texture instead, for ones with no transparent
  space (icons, backgrounds, window frames). Textures that are fully opaque
  are covered anyway.
- `--scale N` makes the picture N times the original's size (2 by default,
  up to 8). Higher is sharper; the game shows it at the same size on screen.
- `--out <folder>` saves somewhere else instead of installing it.

For full control, make the PNG yourself: same proportions as the original
(any multiple of its size), RGBA with transparency where the original has it,
named as `pack-names.txt` says, in the mods folder above.

## When the image doesn't change

**The screen uses a different texture.** The same character often has several
textures: the status screen's hero is `stbustup/st_bu_pc001`, not
`BUSTUPCHAR/BU3D_01_01_01_01`, which looks similar. To find out which
textures a screen really uses, start the program with Zakuro's texture log:

```
set ZAKURO_TEXTURE_HASHES=C:\smt4-textures.txt
target\release\smt4.exe "C:\path\to\Shin Megami Tensei IV (USA).3ds"
```

Go to the screen, close the game, and look up the hashes the log wrote
(`<size> <format> <hash by bytes> <hash as Citra uploaded>`, one line per
texture, in the order they were first drawn) in `pack-names.txt`: the last
hash of a line is the one in the picture names. The textures drawn last are
the ones from the screen you went to last.

`ZAKURO_DUMP_TEXTURES=<folder>` saves every texture drawn as a PNG under its
pack name instead, which also catches textures that aren't in the extracted
files (for example ones inside 3D models).

**The game draws it itself.** Pack pictures only replace textures the game
loads; ones it renders on the fly (some effects, composed screens) can't be.

**Draw the 3D on the GPU is off**, or the picture's name or proportions are
wrong: Zakuro's log says when it can't read a picture.

**Zakuro fell back to drawing in software.** Texture packs need the GPU, and
when Vulkan can't start, Zakuro draws in software and leaves the pack out.
`%APPDATA%\zakuro\zakuro.log` then says `Unable to find a Vulkan driver` and
`the texture pack ... needs the 3D drawn on the GPU, it is left out`. One
cause seen: another program's installer removed the NVIDIA driver's
`VulkanDriverName` from the registry (Windows' `vulkaninfo --summary` says
`Found no drivers!`), which breaks every Vulkan program on the PC.
Reinstalling the graphics driver brings it back; until then, setting
`VK_DRIVER_FILES` to the driver's `nv-vk64.json` (in
`C:\Windows\System32\DriverStore\FileRepository\nv_dispi.inf_amd64_*\`) gets
Vulkan going for one launch.

## Known textures of the hero

| Texture | Size | Where | Pack name |
|---|---|---|---|
| `stbustup/st_bu_pc001` | 256x256 | status screen, battle (his card in the party row) | `tex1_256x256_BDB0F2728E5BE035_13_mip0.png` |
| `faceiconall/icon_pc001` | 32x16 | party bar | `tex1_32x16_762B725D4DD8528B_2_mip0.png` |
| `BUSTUPCHAR/BU3D_01_01_01_01` | 512x512 | not seen yet | `tex1_512x512_BA459DED618AE347_0_mip0.png` |
| `status/st_dot_pc001` | 64x128 | not seen yet | `tex1_64x128_D14064F509C9997B_2_mip0.png` |
| `charicon/icon_pc001` | 32x16 | not seen yet; light-haired, may not be him | `tex1_32x16_3BEF97A6253BD9D2_13_mip0.png` |

The 3D model the hero walks around with has its textures inside the model
(`player/`), not among these; the dump above is the way to find those.

Here's an example

<img width="858" height="1018" alt="image" src="https://github.com/user-attachments/assets/c107b798-81a1-48e8-b0c8-ccf9977d9ae9" />


## Sharing a mod

The pictures you made are yours to share; the game's textures and the
extracted files aren't. A pack of your own pictures under their pack names
works for anyone with the same game (USA), in Zakuro, Citra or Azahar.

## Other kinds of mods

Zakuro also takes files that replace the game's own, at their RomFS paths, in
`%APPDATA%\zakuro\mods\00040000000E5C00\romfs\` (on macOS, under
`~/Library/Application Support/zakuro`), and `.ips`/`.bps` patches to
them in `romfs_ext\`. That's the way for text (`.mbm`), models and sound, but
it needs the files in the game's own formats, and the extractor doesn't write
those back yet.
