#!/usr/bin/env python3
"""Build the pinned viewer with public diagnostic paths; browser qualification follows."""
import argparse
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source', type=Path, required=True, help='clean Mere checkout')
parser.add_argument('--target-dir', type=Path, required=True)
parser.add_argument('--out-dir', type=Path, required=True)
parser.add_argument('--wasm-bindgen', default='wasm-bindgen')
args = parser.parse_args()
source = args.source.resolve()
runtime = '56859e30292405e682f28f3841b76e4f141acf73'
assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=source).strip(), 'source must be clean'
changed = subprocess.check_output(['git', 'diff', '--name-only', runtime, 'HEAD'], cwd=source, text=True).splitlines()
assert all(name.startswith(('design_docs/', 'docs/', 'ports/graphshell/docs/', 'support/doc-audit/')) for name in changed), 'runtime differs from qualified source'
compiler = Path(subprocess.check_output(['rustup', 'which', '--toolchain', '1.99.0', 'rustc'], text=True).strip()).parent
# Preserve the pinned web workspace's target flags when adding public paths.
# CARGO_ENCODED_RUSTFLAGS replaces, rather than extends, configured rustflags.
flags = ['--cfg', 'getrandom_backend="wasm_js"', '-C', 'link-arg=-zstack-size=4194304',
         f'--remap-path-prefix={Path.home()}=/build']
env = dict(os.environ, RUSTC=str(compiler / 'rustc'), RUSTDOC=str(compiler / 'rustdoc'),
           CARGO_ENCODED_RUSTFLAGS='\x1f'.join(flags))
# Explicit toolchain executables need their own dylibs on macOS.
if os.uname().sysname == 'Darwin':
    env['DYLD_LIBRARY_PATH'] = str(compiler.parent / 'lib')
subprocess.run([
    'python3', str(source / 'ports/graphshell/web/scripts/build-viewer.py'),
    '--out-dir', str(args.out_dir.resolve()), '--target-dir', str(args.target_dir.resolve()),
    '--cargo', str(compiler / 'cargo'), '--wasm-bindgen', args.wasm_bindgen, '--offline',
], cwd=source, env=env, check=True)
print('Build complete. Do not replace published bytes without fresh browser qualification and provenance.')
