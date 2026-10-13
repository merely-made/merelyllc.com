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
runtime = '47199232d205576e84c802a84eb0cda401f926e1'
assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=source).strip(), 'source must be clean'
changed = subprocess.check_output(['git', 'diff', '--name-only', runtime, 'HEAD'], cwd=source, text=True).splitlines()
assert all(name.startswith(('design_docs/', 'docs/', 'ports/graphshell/docs/', 'support/doc-audit/')) for name in changed), 'runtime differs from qualified source'
compiler = Path(subprocess.check_output(['rustup', 'which', '--toolchain', '1.98.1', 'rustc'], text=True).strip()).parent
env = dict(os.environ, RUSTC=str(compiler / 'rustc'), RUSTDOC=str(compiler / 'rustdoc'),
           CARGO_ENCODED_RUSTFLAGS=f'--remap-path-prefix={Path.home()}=/build')
# Explicit toolchain executables need their own dylibs on macOS.
if os.uname().sysname == 'Darwin':
    env['DYLD_LIBRARY_PATH'] = str(compiler.parent / 'lib')
subprocess.run([
    'python3', str(source / 'ports/graphshell/web/scripts/build-viewer.py'),
    '--out-dir', str(args.out_dir.resolve()), '--target-dir', str(args.target_dir.resolve()),
    '--cargo', str(compiler / 'cargo'), '--wasm-bindgen', args.wasm_bindgen, '--offline',
], cwd=source, env=env, check=True)
print('Build complete. Do not replace published bytes without fresh browser qualification and provenance.')
