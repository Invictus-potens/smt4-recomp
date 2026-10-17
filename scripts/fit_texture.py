"""Makes a texture pack picture from your own image, to take the place of one
of the game's textures. See docs/MODDING.md.

    python scripts/fit_texture.py <extracted PNG> <your image> [--scale 2] [--fill]

<extracted PNG> is the texture to replace, as tools/extract wrote it (under
<assets>/textures/). Your image is fitted into the area the original's
visible pixels take up (a character inside a mostly transparent texture
stays where and as big as the game expects it), keeping its proportions,
centered across and resting on the bottom of that area, with the rest left
transparent. --fill covers the whole texture instead, for ones with no
transparent space, like icons and backgrounds.

The picture is --scale times the original's size (2 by default, up to 8),
for more detail than the game's own, and is saved under the name Zakuro
looks for, from the pack-names.txt next to the extracted textures, in
Zakuro's mods folder for the game. Restart the game to see it.

Needs ffmpeg on PATH.
"""
import argparse
import os
import pathlib
import re
import shutil
import subprocess
import sys

TITLE_ID = '00040000000E5C00'


def fail(text):
    print(f'error: {text}', file=sys.stderr)
    sys.exit(1)


def ffmpeg(*args):
    result = subprocess.run(['ffmpeg', '-hide_banner', *args], capture_output=True, text=True)
    return result


def size_of(path):
    out = subprocess.run(['ffprobe', '-v', 'error', '-select_streams', 'v:0', '-show_entries', 'stream=width,height',
                          '-of', 'csv=p=0', str(path)], capture_output=True, text=True).stdout.strip()
    if not out:
        fail(f'could not read {path}')
    width, height = out.split(',')[:2]
    return int(width), int(height)


def visible_area(path):
    """the box the original's visible pixels take up, x, y, width, height"""
    log = ffmpeg('-i', str(path), '-vf', 'alphaextract,bbox=min_val=16', '-f', 'null', '-').stderr
    boxes = re.findall(r'x1:(\d+) x2:(\d+) y1:(\d+) y2:(\d+)', log)
    if not boxes:
        return None
    x1, x2, y1, y2 = map(int, boxes[-1])
    return x1, y1, x2 - x1 + 1, y2 - y1 + 1


def pack_name(extracted):
    """the picture's name from the pack-names.txt above the extracted PNG"""
    for folder in extracted.resolve().parents:
        names = folder / 'pack-names.txt'
        if names.is_file():
            relative = extracted.resolve().relative_to(folder).as_posix()
            for line in names.read_text(encoding='utf-8').splitlines():
                if '\t' in line and line.split('\t')[0] == relative:
                    return line.split('\t')[1]
            fail(f'{relative} is not in {names}; extract the textures again with tools/extract')
    fail(f'no pack-names.txt above {extracted}; extract the textures with tools/extract')


def mods_folder():
    if os.name == 'nt':
        base = pathlib.Path(os.environ['APPDATA']) / 'zakuro'
    else:
        base = pathlib.Path(os.environ.get('XDG_DATA_HOME', pathlib.Path.home() / '.local' / 'share')) / 'zakuro'
    return base / 'mods' / TITLE_ID / 'textures'


def main():
    doc = (__doc__ or '').splitlines()
    parser = argparse.ArgumentParser(description=doc[0], formatter_class=argparse.RawDescriptionHelpFormatter,
                                     epilog='\n'.join(doc[2:]))
    parser.add_argument('extracted', type=pathlib.Path, help='the extracted PNG of the texture to replace')
    parser.add_argument('image', type=pathlib.Path, help='your image')
    parser.add_argument('--scale', type=int, default=2, help='times the original size, 1 to 8 (default 2)')
    parser.add_argument('--fill', action='store_true', help='cover the whole texture instead of the visible area')
    parser.add_argument('--out', type=pathlib.Path, help="folder to save in instead of Zakuro's mods folder")
    args = parser.parse_args()

    if not shutil.which('ffmpeg'):
        fail('ffmpeg is not on PATH')
    for path in (args.extracted, args.image):
        if not path.is_file():
            fail(f'{path} not found')
    scale = max(1, min(8, args.scale))
    name = pack_name(args.extracted)
    width, height = size_of(args.extracted)
    area = None if args.fill else visible_area(args.extracted)
    if area is None or area[2:] == (width, height):
        area = (0, 0, width, height)
        fit = 'increase'  # covers it, cropping what's over
    else:
        fit = 'decrease'  # fits inside, leaving the rest transparent
    x, y, w, h = (v * scale for v in area)
    canvas = f'{width * scale}x{height * scale}'

    out_dir = args.out or mods_folder()
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / name
    graph = (
        f'color=c=black@0.0:s={canvas},format=rgba[canvas];'
        f'[0:v]scale={w}:{h}:force_original_aspect_ratio={fit}:flags=lanczos,crop=min(iw\\,{w}):min(ih\\,{h}),format=rgba[image];'
        f'[canvas][image]overlay=x={x}+({w}-overlay_w)/2:y={y}+{h}-overlay_h:format=auto'
    )
    result = ffmpeg('-loglevel', 'error', '-y', '-i', str(args.image), '-filter_complex', graph,
                    '-frames:v', '1', '-update', '1', str(out))
    if result.returncode != 0:
        fail(result.stderr.strip())
    kind = 'the whole texture' if fit == 'increase' else f'its visible area, {area[2]}x{area[3]} at {area[0]},{area[1]}'
    print(f'{args.image.name} fitted into {kind}, at {canvas}')
    print(f'saved as {out}')
    print('restart the game to see it')


if __name__ == '__main__':
    main()
