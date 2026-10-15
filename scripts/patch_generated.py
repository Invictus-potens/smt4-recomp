"""Patches 3dsrecomp's generated C at sites that aren't function entries, so a
RECOMP_OVERRIDE can't reach them. Run after every `3dsrecomp port`, from the
project root:

    python scripts/patch_generated.py --rebuild

Each patch is anchored on the instruction comment 3dsrecomp writes before the
code for each ARM instruction ("/* ADDRESS OPCODE */"), so it fails loudly
instead of patching the wrong place if a new port moves things around.
Re-running is a no-op.

--rebuild recompiles the patched files with the flags 3dsrecomp uses and
puts them into code/librecomp.a, so `cargo build` picks them up. CC and AR
pick the compiler and archiver (default gcc and ar, MinGW-w64's on Windows).
"""
import argparse
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CODE = ROOT / 'code'

# the flags 3dsrecomp compiles generated code with (src/compile.rs)
CFLAGS = ['-O2', '-ffp-contract=off', '-fno-math-errno', '-w']

DECLS = [
    'extern uint32_t smt4_boost_exp(Context *ctx, uint32_t gain);\n',
    'extern uint32_t smt4_boost_demon_exp(Context *ctx, uint32_t address);\n',
    'extern uint32_t smt4_widen_aspect(Context *ctx, uint32_t bits);\n',
]

# (description, instruction comment, text right after it, replacement)
PATCHES = [
    (
        'hero EXP from battle results: hero.exp(r1) + gain(r0) at 0x0018CF84',
        '/* 0018CF84 E0817000 */',
        '    uint32_t m = reg0_;',
        '    uint32_t m = smt4_boost_exp(ctx, reg0_); /* smt4-recomp: EXP multiplier */',
    ),
    (
        # the gain is boosted where it's loaded, and written back, because
        # 0x0018D230 later re-reads it through a pointer to undo the add
        'demon EXP from battle results: load of results[0x35B8 + id*4] at 0x0018D190',
        '/* 0018D190 E5900000 */',
        '    uint32_t v = mem_read32(ctx, oa);',
        '    uint32_t v = smt4_boost_demon_exp(ctx, oa); /* smt4-recomp: EXP multiplier */',
    ),
    (
        'widescreen: aspect (s1) on entry to the perspective builder at 0x002B761C',
        '/* 002B761C E92D4010 */',
        '    { uint32_t base = reg13_, p = base - 8, fin = base - 8;',
        '    vfp1_ = smt4_widen_aspect(ctx, vfp1_); /* smt4-recomp: widescreen */\n'
        '    { uint32_t base = reg13_, p = base - 8, fin = base - 8;',
    ),
    (
        'widescreen: aspect (s3) on entry to the camera\'s SetPerspective at 0x002C1FFC',
        '/* 002C1FFC E92D4070 */',
        '    { uint32_t base = reg13_, p = base - 16, fin = base - 16;',
        '    vfp3_ = smt4_widen_aspect(ctx, vfp3_); /* smt4-recomp: widescreen */\n'
        '    { uint32_t base = reg13_, p = base - 16, fin = base - 16;',
    ),
]


def find_file(anchor: str) -> pathlib.Path:
    hits = [p for p in sorted(CODE.glob('code*.c')) if anchor in p.read_text(errors='ignore')]
    if len(hits) != 1:
        sys.exit(f'anchor {anchor!r} found in {len(hits)} files, expected 1')
    return hits[0]


def patch() -> tuple:
    """applies every patch; returns the files patched at all, and those
    changed by this run"""
    patched, changed = set(), set()
    for desc, anchor, old, new in PATCHES:
        path = find_file(anchor)
        text = path.read_text(errors='ignore')
        at = text.index(anchor)
        window_end = text.index('/*', at + len(anchor))  # next instruction comment
        window = text[at:window_end]
        marker = new.split('/*')[0].strip()
        patched.add(path)
        if marker in text[at:at + len(anchor) + 400]:
            print(f'already patched: {desc} ({path.name})')
            continue
        if window.count(old) != 1:
            sys.exit(f'{desc}: expected exactly one {old!r} after {anchor} in {path.name}')
        text = text[:at] + window.replace(old, new) + text[window_end:]
        missing = ''.join(d for d in DECLS if d not in text)
        if missing:
            first = text.index('\nvoid f_') + 1  # before the first generated function
            text = text[:first] + missing + text[first:]
        path.write_text(text)
        changed.add(path)
        print(f'patched: {desc} ({path.name})')
    return patched, changed


def rebuild(files: set) -> None:
    cc = os.environ.get('CC', 'gcc')
    ar = os.environ.get('AR', 'ar')
    # overrides/*.c are recompiled too, so editing an override's body doesn't
    # need a new port. An override at a *new* address still does: port is
    # what points the callers and the entry table at it.
    sources = sorted(files) + sorted((ROOT / 'overrides').glob('*.c'))
    objects = []
    for path in sources:
        obj = CODE / (path.stem + '.o')
        print(f'compiling {path.name}')
        subprocess.run([cc, *CFLAGS, '-I', str(CODE), '-c', str(path), '-o', str(obj)], check=True)
        objects.append(obj)
    # archive members are named after the file, so this replaces the
    # objects 3dsrecomp put there
    subprocess.run([ar, 'r', str(CODE / 'librecomp.a'), *map(str, objects)], check=True)
    for obj in objects:
        obj.unlink()
    print(f'updated librecomp.a with {len(objects)} object(s)')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--rebuild', action='store_true', help='recompile patched files into code/librecomp.a')
    args = parser.parse_args()
    files, changed = patch()
    if args.rebuild:
        rebuild(files)
    elif changed:
        print('run with --rebuild to compile:', ' '.join(sorted(p.name for p in changed)))


if __name__ == '__main__':
    main()
