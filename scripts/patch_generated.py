"""Patches 3dsrecomp's generated C at sites that aren't function entries, so a
RECOMP_OVERRIDE can't reach them. Run after every `3dsrecomp port`, from the
project root:

    python scripts/patch_generated.py

Each patch is anchored on the instruction comment 3dsrecomp writes before the
code for each ARM instruction ("/* ADDRESS OPCODE */"), so it fails loudly
instead of patching the wrong place if a new port moves things around.
Re-running is a no-op.
"""
import glob
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CODE = ROOT / 'code'

DECL = 'extern uint32_t smt4_boost_exp(Context *ctx, uint32_t gain);\n'

# (description, instruction comment, text right after it, replacement)
PATCHES = [
    (
        'hero EXP from battle results: hero.exp(r1) + gain(r0) at 0x0018CF84',
        '/* 0018CF84 E0817000 */',
        '    uint32_t m = reg0_;',
        '    uint32_t m = smt4_boost_exp(ctx, reg0_); /* smt4-recomp: EXP multiplier */',
    ),
]


def find_file(anchor: str) -> pathlib.Path:
    hits = [p for p in sorted(CODE.glob('code*.c')) if anchor in p.read_text(errors='ignore')]
    if len(hits) != 1:
        sys.exit(f'anchor {anchor!r} found in {len(hits)} files, expected 1')
    return hits[0]


def main() -> None:
    changed = set()
    for desc, anchor, old, new in PATCHES:
        path = find_file(anchor)
        text = path.read_text(errors='ignore')
        at = text.index(anchor)
        window_end = text.index('/*', at + len(anchor))  # next instruction comment
        window = text[at:window_end]
        marker = new.split('/*')[0].strip()
        if marker in text[at:at + len(anchor) + 400]:
            print(f'already patched: {desc} ({path.name})')
            continue
        if window.count(old) != 1:
            sys.exit(f'{desc}: expected exactly one {old!r} after {anchor} in {path.name}')
        text = text[:at] + window.replace(old, new) + text[window_end:]
        if DECL not in text:
            first = text.index('\nvoid f_') + 1  # before the first generated function
            text = text[:first] + DECL + text[first:]
        path.write_text(text)
        changed.add(path.name)
        print(f'patched: {desc} ({path.name})')
    if changed:
        print('recompile:', ' '.join(sorted(changed)))


if __name__ == '__main__':
    main()
