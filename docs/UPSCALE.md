# Upscaled portraits

The game's portraits (demons, characters, bosses) can be redrawn at 4x their
size with an AI upscaler and installed as a texture pack, so they stay sharp
in a big window. Everything is made on your PC from your own copy: the
upscaled pictures are still the game's art and must not be shared.

**Status:** tested on all 889 portraits (731 pictures once duplicates are
merged). A tool that does it in one command is planned (see the end).

## What was done

1. **Extract** the textures with `smt4-extract` (see [ASSETS.md](ASSETS.md)).
   `textures/pack-names.txt` gives each one's texture pack name.
2. **Upscale** with
   [Real-ESRGAN ncnn-vulkan](https://github.com/xinntao/Real-ESRGAN/releases)
   (v0.2.5.0, a portable `.exe` that runs on the GPU through Vulkan, no
   Python or CUDA), model `realesrgan-x4plus`, 4x:
   ```
   realesrgan-ncnn-vulkan.exe -i <folder of PNGs> -o <out folder> -n realesrgan-x4plus -s 4 -f png
   ```
   Transparency comes through as it was, with no fringes.
3. **Size:** 256 px portraits are kept at 4x (1024 px). Those 512 px and
   wider are done at 4x and brought down to 2x with Lanczos, so the pack
   stays around 600 MB.
4. **Install** each one under its pack name in
   `%APPDATA%\zakuro\mods\00040000000E5C00\textures\`, leaving pictures
   already there (your own mods) alone.

On an RTX 2060 the 889 portraits took about 12 minutes.

Folders upscaled: `tex/stex/stbustup/` (demon portraits, `st_bu_en*`),
`tex/stex/BUSTUPCHAR/` (characters) and `tex/stex/bustup/` (bosses and other
characters).

## Choosing the model

Compared on several demons, over a gray background:

| Model | Result |
|---|---|
| `realesrgan-x4plus-anime` | sharpest lines, but the painted shading turns into flat colors; furthest from the original art |
| **`realesrgan-x4plus`** | sharp and keeps the painted shading; **the default** |
| `realesr-animevideov3` | sharp but a bit smeared, more artifacts at the edges |

`realesrgan-x4plus` smooths very fine detail as noise, so some faces came
out soft, mostly the Megami that share the same face style. For those, these
were compared: x4plus with an unsharp filter, x4plus mixed 70/30 with a plain
Lanczos enlargement, and three community models in the same ncnn format
(from [Upscayl](https://github.com/upscayl/upscayl)): **UltraSharp**,
Remacri and High-Fidelity. **UltraSharp** kept eyes, mouths and hair strands
sharpest with no halos, and replaced x4plus for 14 portraits:

```
st_bu_en011 st_bu_en012 st_bu_en013 st_bu_en049 st_bu_en052 st_bu_en053
st_bu_en067 st_bu_en074 st_bu_en100 st_bu_en162 st_bu_en167 st_bu_en170
st_bu_en173 st_bu_en468
```

## Checking the result

A page with every portrait before and after, side by side, with a box to tick
the ones that came out wrong, made the review quick: looking at all 726
portraits in the game's compendium would take much longer.

## Next

- The rest of the textures: UI, backgrounds, models and maps. Model textures
  are mostly ETC1, a lossy format, where an upscaler can also sharpen the
  compression's blocks; text and fonts may need another model or none.
- A tool in this repository, something like `cargo upscale-pack <rom>`, that
  does it all on the player's PC: extract, upscale by group
  (`--only portraits,ui,models`), pick the model per group, install, and the
  review page.
- Later: a mod tool to pick a demon and give it a picture of your own, fitted
  and named for the pack (as `cargo fit-texture` does today for one texture).
