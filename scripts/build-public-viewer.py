#!/usr/bin/env python3
"""Build the pinned viewer from Turnstone's web port and install it under assets/graphshell.

The viewer's source is Turnstone's `ports/graphshell/web` (it left Mere with the engine
split), which takes Graphshell from branch main. Turnstone tracks no Cargo.lock, so the
revisions cargo resolved are read from the lock the build leaves and recorded in
provenance.json. Cargo runs through the workspace build-storage wrapper on one lane.
Browser qualification follows: the site's headed smoke is the gate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile

SITE = Path(__file__).resolve().parents[1]
ASSETS = SITE / 'assets/graphshell'
STORAGE = Path('/Users/markik/Code/tools/build_storage.py')
TOOLCHAIN = '1.99.1'
# Published files: site path -> path in the Turnstone web directory or the build output.
WEB_FILES = ('viewer.js', 'mount.js', 'styles.css')
BUILT_FILES = (
    'webgl2/graphshell_web.js',
    'webgl2/graphshell_web_bg.wasm',
    'physics/graphshell_physics.js',
    'physics/graphshell_physics_bg.wasm',
    'physics/worker.js',
)
REPO = re.compile(r'git\+https://github\.com/merely-made/([a-z-]+)\.git\?branch=main#([0-9a-f]{40})')


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cargo_shim(directory, lane):
    """A `cargo` for build-viewer.py that goes through the wrapper and drops --target-dir."""
    shim = directory / 'cargo-via-wrapper'
    shim.write_text(f'''#!/usr/bin/env python3
import os, subprocess, sys
args, out, skip = sys.argv[1:], [], False
for arg in args:
    if skip:
        skip = False
    elif arg == '--target-dir':
        skip = True
    else:
        out.append(arg)
sys.exit(subprocess.call(['python3', {str(STORAGE)!r}, 'build', '--lane', {lane!r},
                          '--cwd', os.getcwd(), '--', '+{TOOLCHAIN}', *out]))
''')
    shim.chmod(shim.stat().st_mode | stat.S_IXUSR)
    return shim


def build(args, source, web, out):
    flags = ['--cfg', 'getrandom_backend="wasm_js"', '-C', 'link-arg=-zstack-size=4194304',
             f'--remap-path-prefix={Path.home()}=/build']
    # CARGO_ENCODED_RUSTFLAGS replaces the web port's configured target rustflags, so it repeats them.
    env = dict(os.environ, CARGO_ENCODED_RUSTFLAGS='\x1f'.join(flags))
    with tempfile.TemporaryDirectory() as scratch:
        subprocess.run([
            sys.executable, str(web / 'scripts/build-viewer.py'), '--out-dir', str(out),
            '--target-dir', str(args.target_dir.resolve()),
            '--cargo', str(cargo_shim(Path(scratch), args.lane)),
            '--wasm-bindgen', args.wasm_bindgen,
            '--terser', str(args.terser or web / 'scripts/node_modules/.bin/terser'),
        ], cwd=source, env=env, check=True)
    return flags


def install(source, web, out, flags):
    delivery = json.loads((out / 'viewer-delivery.json').read_text())
    assert delivery['all_caps_pass'], 'the build reported a delivery cap failure'
    snippets = sorted(p.relative_to(out / 'webgl2').as_posix() for p in (out / 'webgl2/snippets').rglob('*.js'))
    built = list(BUILT_FILES) + [f'webgl2/{name}' for name in snippets]
    if ASSETS.exists():
        shutil.rmtree(ASSETS)
    files = {}
    for name in WEB_FILES:
        files[name] = (web / name, name)
    for name in built:
        files[f'pkg/{name}'] = (out / name, f'pkg/{name}')
    for name, (src, dest) in files.items():
        (ASSETS / dest).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, ASSETS / dest)
    lock = (web / 'Cargo.lock').read_text()
    sources = {}
    for name, rev in REPO.findall(lock):
        sources.setdefault(name, rev)
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=source, text=True).strip()
    # The delivery report asks the checkout's own rustc; the wrapper builds with TOOLCHAIN.
    rustc = subprocess.check_output(['rustc', f'+{TOOLCHAIN}', '--version'], text=True).split(' (')[0]
    sizes = {key: {'gzip6': row['gzip6_bytes'], 'gzip9': row['gzip9_bytes'], 'cap': row['cap_bytes']}
             for key, row in delivery['sizes'].items()}
    provenance = {
        'runtime_source': head,
        'runtime_repository': 'turnstone ports/graphshell/web',
        'graphshell_source': sources['graphshell'],
        'dependency_sources': dict(sorted(sources.items())),
        'resolved_lock_sha256': hashlib.sha256(lock.encode()).hexdigest(),
        'files': {name: sha256(ASSETS / name) for name in files},
        'artifacts': delivery['artifacts'],
        'toolchain': f"{rustc}; wasm-bindgen 0.2.129; terser {delivery['minifier_version'].split()[-1]}; "
                     'viewer profile; no default features; engine-free',
        'rustflags': flags,
        'sizes': sizes,
        'build_adjustment': 'Cargo through the build-storage wrapper; target dir and rustflags as scripts/build-public-viewer.py sets them.',
        'previous_published_runtime_source': '14b91e3df2e239924bdb3d7b88a58d00f40871ae',
        'previous_layout': 'Mere ports/graphshell/web, one pkg/graphshell_web wasm; the viewer source left Mere in the engine split (U4).',
    }
    (ASSETS / 'provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
    return provenance


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True, help='clean Turnstone checkout')
    parser.add_argument('--lane', required=True, help='build-storage lane for the Cargo runs')
    parser.add_argument('--target-dir', type=Path, required=True, help="the lane's artifacts directory")
    parser.add_argument('--out-dir', type=Path, required=True)
    parser.add_argument('--wasm-bindgen', default='wasm-bindgen')
    parser.add_argument('--terser', type=Path, help='defaults to the web port scripts/node_modules/.bin/terser')
    parser.add_argument('--skip-build', action='store_true', help='install an existing --out-dir build')
    args = parser.parse_args()
    source = args.source.resolve()
    web = source / 'ports/graphshell/web'
    assert not subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=source).strip(), 'source must be clean'
    flags = ['--cfg', 'getrandom_backend="wasm_js"', '-C', 'link-arg=-zstack-size=4194304',
             '--remap-path-prefix=<build-host-home>=/build']
    if not args.skip_build:
        build(args, source, web, args.out_dir.resolve())
    provenance = install(source, web, args.out_dir.resolve(), flags)
    print(json.dumps({'installed': provenance['runtime_source'], 'graphshell': provenance['graphshell_source'],
                      'sizes': provenance['sizes']}, indent=2))
    print('Installed. Do not publish without fresh browser qualification (the Pages smoke).')


if __name__ == '__main__':
    main()
