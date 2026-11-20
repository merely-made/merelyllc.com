#!/usr/bin/env python3
"""Verify the immutable, browser-qualified viewer and its delivery ceilings."""
import gzip
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
assets = root / 'assets/graphshell'
provenance = json.loads((assets / 'provenance.json').read_text())
WEBGL2_WASM = 'pkg/webgl2/graphshell_web_bg.wasm'
PHYSICS_WASM = 'pkg/physics/graphshell_physics_bg.wasm'
GLUE = ('pkg/webgl2/graphshell_web.js', 'pkg/physics/graphshell_physics.js', 'pkg/physics/worker.js')
MOUNTING = ('viewer.js', 'mount.js', 'styles.css')
files = provenance['files']
assert provenance['runtime_source'] == 'd3dd2c177b45d4ecd5fc79e181716d08501a2e8c'
assert provenance['graphshell_source'] == '2b0b5ff4a512704c9690256735779abd7ab12fbb'
assert files[WEBGL2_WASM] == '61daf58315098d57e26b2e6555f7a3b4b813c6e42b79ab254a052a05951ed4cc'
assert files[PHYSICS_WASM] == 'eae38540ca1a55f7989c3bf968e06210dccc9ff0a6e174e87f7de9528afe6f3b'
for name, expected in files.items():
    assert hashlib.sha256((assets / name).read_bytes()).hexdigest() == expected, name
published = {p.relative_to(assets).as_posix() for p in assets.rglob('*') if p.is_file()}
assert published == set(files) | {'provenance.json'}, sorted(published ^ (set(files) | {'provenance.json'}))
viewer_rs = (root / 'src/viewer.rs').read_text()
assert files[WEBGL2_WASM] in viewer_rs and files[PHYSICS_WASM] in viewer_rs
for name in published:
    assert f'"{name}"' in viewer_rs, name
for name in (WEBGL2_WASM, PHYSICS_WASM):
    for marker in (b'/Users/', b'/home/', b'C:\\Users\\'):
        assert marker.lower() not in (assets / name).read_bytes().lower(), name
def gzipped(path, level):
    return len(gzip.compress(path.read_bytes(), compresslevel=level, mtime=0))
for level in (6, 9):
    sizes = {name: gzipped(assets / name, level) for name in files}
    assert sizes[WEBGL2_WASM] <= 3_407_872, sizes
    assert sizes[PHYSICS_WASM] <= 655_360, sizes
    for name in GLUE:
        assert sizes[name] <= 20_480, sizes
    mounting = sum(sizes[n] for n in MOUNTING) + gzipped(root / 'assets/graph-sandbox-mount.js', level)
    assert mounting <= 10240, mounting
    print(json.dumps({'gzip_level': level, 'files': sizes, 'mount_and_styles': mounting}))
