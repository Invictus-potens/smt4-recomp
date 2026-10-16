"""Builds smt4-recomp from your own copy of Shin Megami Tensei IV (USA).

    python build.py <path to your .3ds or .cci>

Does every step in order, skipping the ones already done:

1. checks Rust, a C compiler, ar and git are there;
2. checks the ROM is Shin Megami Tensei IV (USA), title 00040000000E5C00;
3. fetches and builds 3dsrecomp, at the version this project was tested with;
4. recompiles the game's code into C with this project's overrides (the slow
   step, 20 minutes or more), into code/;
5. patches the generated C (scripts/patch_generated.py);
6. builds the program, target/release/smt4(.exe);
7. puts the cheats file in Zakuro's cheats folder, backing up one already
   there.

Nothing it makes is meant to be shared: code/ and the program hold the game's
code. Options: --force redoes step 4 even when nothing changed,
--no-cheats skips step 7, --low-priority builds at a lower priority, to keep
playing something meanwhile. CC and AR pick the C compiler and archiver.
"""
import argparse
import hashlib
import os
import pathlib
import platform
import shutil
import subprocess
import sys
from typing import NoReturn

ROOT = pathlib.Path(__file__).resolve().parent
WORK = ROOT / '.build'
TITLE_ID = 0x00040000000E5C00
RECOMP_URL = 'https://github.com/fearkov/3dsrecomp'
# the 3dsrecomp version the overrides and patches were made and tested with
RECOMP_REV = 'c3f21f9'
NAME = 'smt4'
EXE = '.exe' if os.name == 'nt' else ''


def step(text: str) -> None:
    print(f'\n== {text}', flush=True)


def fail(text: str) -> NoReturn:
    print(f'\nerror: {text}', file=sys.stderr)
    sys.exit(1)


def run(args: list, cwd: pathlib.Path = ROOT, env: dict | None = None) -> None:
    print('>', ' '.join(str(a) for a in args), flush=True)
    if subprocess.run([str(a) for a in args], cwd=cwd, env=env).returncode != 0:
        fail(f'{pathlib.Path(str(args[0])).name} failed, see above')


def output(args: list) -> str:
    try:
        return subprocess.run(args, capture_output=True, text=True).stdout.strip()
    except OSError:
        return ''


def check_tools() -> dict:
    step('1. tools')
    for tool, hint in (('cargo', 'install Rust from https://rustup.rs'),
                       ('git', 'install git from https://git-scm.com')):
        if not shutil.which(tool):
            fail(f'{tool} not found: {hint}')
    cc = os.environ.get('CC', 'gcc')
    ar = os.environ.get('AR', 'ar')
    for tool in (cc, ar):
        if not shutil.which(tool):
            fail(f'{tool} not found. On Windows, install MinGW-w64 GCC, for example with '
                 '"winget install BrechtSanders.WinLibs.POSIX.UCRT", or set CC and AR to it')
    machine = output([cc, '-dumpmachine'])
    host = output(['rustc', '-vV'])
    host = next((l.split(': ')[1] for l in host.splitlines() if l.startswith('host: ')), '')
    print(f'C compiler: {shutil.which(cc)} ({machine or "?"})')
    print(f'Rust: {host or "?"}')
    if machine and not machine.startswith('x86_64') and platform.machine().lower() in ('amd64', 'x86_64'):
        fail(f'{cc} builds for {machine}, not 64-bit: another, older gcc is probably first on PATH. '
             'Set CC and AR to the full paths of a 64-bit MinGW-w64 gcc and ar')
    if os.name == 'nt' and host.endswith('msvc'):
        fail('Rust uses the MSVC toolchain, but the C side is built with MinGW-w64 here. Run '
             '"rustup default stable-x86_64-pc-windows-gnu" and try again')
    env = dict(os.environ, CC=cc, AR=ar)
    # 3dsrecomp's rust-toolchain.toml asks for "stable", which on Windows
    # means the MSVC one even when the GNU one is the default: build it with
    # the same toolchain as this project
    toolchain = output(['rustup', 'show', 'active-toolchain']).split(' ')[0] if shutil.which('rustup') else ''
    if toolchain:
        env['RUSTUP_TOOLCHAIN'] = toolchain
        print(f'Rust toolchain: {toolchain}')
    return env


def check_rom(rom: pathlib.Path) -> None:
    step('2. ROM')
    if not rom.is_file():
        fail(f'{rom} not found')
    with rom.open('rb') as f:
        header = f.read(0x200)
    if header[0x100:0x104] == b'NCSD':
        title = int.from_bytes(header[0x108:0x110], 'little')
    elif header[0x100:0x104] == b'NCCH':
        title = int.from_bytes(header[0x118:0x120], 'little')
    else:
        title = None
        fail(f'{rom.name} is not a decrypted .3ds/.cci/.cxi (no NCSD or NCCH header)')
    if title != TITLE_ID:
        fail(f'{rom.name} is title {title:016X}, this project is for Shin Megami Tensei IV (USA), '
             f'{TITLE_ID:016X}')
    print(f'{rom.name}: Shin Megami Tensei IV (USA), {rom.stat().st_size / 2**30:.2f} GiB')


def build_3dsrecomp(env: dict) -> pathlib.Path:
    step(f'3. 3dsrecomp ({RECOMP_REV})')
    src = WORK / '3dsrecomp'
    if not (src / '.git').is_dir():
        WORK.mkdir(exist_ok=True)
        run(['git', 'clone', '--quiet', RECOMP_URL, src])
    if not output(['git', '-C', str(src), 'rev-parse', '--verify', '--quiet', f'{RECOMP_REV}^{{commit}}']):
        run(['git', 'fetch', '--quiet', 'origin'], cwd=src)
    run(['git', 'checkout', '--quiet', RECOMP_REV], cwd=src)
    run(['cargo', 'build', '--release', '--quiet'], cwd=src, env=env)
    return src / 'target' / 'release' / f'3dsrecomp{EXE}'


def inputs_stamp(rom: pathlib.Path) -> str:
    """what step 4's output depends on"""
    h = hashlib.sha256()
    h.update(RECOMP_REV.encode())
    h.update(str(rom.stat().st_size).encode())
    for path in sorted((ROOT / 'overrides').glob('*.[ch]')):
        h.update(path.name.encode())
        h.update(path.read_bytes())
    return h.hexdigest()


def port(recomp: pathlib.Path, rom: pathlib.Path, env: dict, force: bool) -> None:
    step('4. recompiling the game\'s code')
    code = ROOT / 'code'
    stamp_file = code / '.smt4-stamp'
    stamp = inputs_stamp(rom)
    if not force and (code / 'librecomp.a').is_file() and stamp_file.is_file() \
            and stamp_file.read_text().strip() == stamp:
        print('code/ is up to date, skipping (--force redoes it)')
        return
    print('This takes 20 minutes or more, and a few GB of memory.')
    out = WORK / 'port'
    shutil.rmtree(out, ignore_errors=True)
    run([recomp, 'port', rom.resolve(), out.resolve(), '--name', NAME,
         '--overrides', (ROOT / 'overrides').resolve()], env=env)
    if not (out / 'code' / 'librecomp.a').is_file():
        fail(f'3dsrecomp port did not write {out / "code" / "librecomp.a"}')
    # only code/ is kept: port also writes a Cargo project of its own, and
    # this one's src/main.rs defines things the overrides use
    shutil.rmtree(code, ignore_errors=True)
    shutil.move(str(out / 'code'), str(code))
    shutil.rmtree(out, ignore_errors=True)
    stamp_file.write_text(stamp + '\n')


def patch(env: dict) -> None:
    step('5. patching the generated C')
    run([sys.executable, ROOT / 'scripts' / 'patch_generated.py', '--rebuild'], env=env)


def build(env: dict) -> pathlib.Path:
    step('6. building the program')
    run(['cargo', 'build', '--release'], env=env)
    exe = ROOT / 'target' / 'release' / f'{NAME}{EXE}'
    if not exe.is_file():
        fail(f'{exe} was not built')
    return exe


def zakuro_dir() -> pathlib.Path:
    if os.name == 'nt':
        return pathlib.Path(os.environ['APPDATA']) / 'zakuro'
    if sys.platform == 'darwin':
        return pathlib.Path.home() / 'Library' / 'Application Support' / 'zakuro'
    return pathlib.Path(os.environ.get('XDG_DATA_HOME', pathlib.Path.home() / '.local' / 'share')) / 'zakuro'


def install_cheats() -> None:
    step('7. cheats')
    name = f'{TITLE_ID:016X}.txt'
    source = ROOT / 'cheats' / name
    target = zakuro_dir() / 'cheats' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.is_file():
        if target.read_bytes() == source.read_bytes():
            print(f'{target} is up to date')
            return
        backup = target.with_suffix('.txt.bak')
        shutil.copy2(target, backup)
        print(f'backed up the cheats file that was there to {backup}')
    shutil.copy2(source, target)
    print(f'copied to {target}')


def lower_priority() -> None:
    """the compilers started from here inherit it"""
    if os.name == 'nt':
        import ctypes
        from ctypes import wintypes
        below_normal = 0x4000
        kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
        # a HANDLE is 64 bits: without these, ctypes cuts it to an int
        kernel32.GetCurrentProcess.restype = wintypes.HANDLE
        kernel32.SetPriorityClass.argtypes = (wintypes.HANDLE, wintypes.DWORD)
        kernel32.SetPriorityClass.restype = wintypes.BOOL
        if not kernel32.SetPriorityClass(kernel32.GetCurrentProcess(), below_normal):
            print(f'could not lower the priority (error {ctypes.get_last_error()})')
    else:
        os.nice(10)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0],
                                     formatter_class=argparse.RawDescriptionHelpFormatter,
                                     epilog='\n'.join(__doc__.splitlines()[2:]))
    parser.add_argument('rom', type=pathlib.Path, help='your Shin Megami Tensei IV (USA) .3ds/.cci')
    parser.add_argument('--force', action='store_true', help='redo the recompilation even when nothing changed')
    parser.add_argument('--no-cheats', action='store_true', help="don't copy the cheats file to Zakuro's folder")
    parser.add_argument('--low-priority', action='store_true', help='build at a lower priority')
    args = parser.parse_args()
    if args.low_priority:
        lower_priority()

    env = check_tools()
    check_rom(args.rom)
    recomp = build_3dsrecomp(env)
    port(recomp, args.rom, env, args.force)
    patch(env)
    exe = build(env)
    if not args.no_cheats:
        install_cheats()
    step('done')
    print(f'Run it with:\n  "{exe}" "{args.rom.resolve()}"')
    print('Recommended: Settings, Layout "Top screen only", Top screen shape 16:9, and the '
          '"Widescreen 16:9" cheat. See README.md.')


if __name__ == '__main__':
    main()
