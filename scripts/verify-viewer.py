#!/usr/bin/env python3
"""Verify the immutable, browser-qualified viewer and its delivery ceilings."""
import gzip
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
assets = root / 'assets/graphshell'
provenance = json.loads((assets / 'provenance.json').read_text())
assert provenance['runtime_source'] == '25c4eae566fdb81571f58201799adae6f0244aeb'
assert provenance['files']['pkg/graphshell_web_bg.wasm'] == 'a988b4caad5376c9c06020d15c45b809d673afc349dad920a08d6ecc4d8e39ce'
assert provenance['files']['pkg/graphshell_web.js'] == '5e1dc255576b19c64fe6b3e99ba7071f44662f668c939360b9c3c70c3cb1da4a'
for name, expected in provenance['files'].items():
    assert hashlib.sha256((assets / name).read_bytes()).hexdigest() == expected, name
assert provenance['files']['pkg/graphshell_web_bg.wasm'] in (root / 'src/viewer.rs').read_text()
for marker in (b'/Users/', b'/home/', b'C:\\Users\\'):
    assert marker.lower() not in (assets / 'pkg/graphshell_web_bg.wasm').read_bytes().lower()
for level in (6, 9):
    sizes = {name: len(gzip.compress((assets / name).read_bytes(), compresslevel=level, mtime=0)) for name in provenance['files']}
    assert sizes['pkg/graphshell_web_bg.wasm'] <= 3_407_872, sizes
    assert sizes['pkg/graphshell_web.js'] <= 20_480, sizes
    mounting = sum(sizes[n] for n in ('viewer.js', 'mount.js', 'styles.css')) + len(gzip.compress((root / 'assets/graph-sandbox-mount.js').read_bytes(), compresslevel=level, mtime=0))
    assert mounting <= 8192, mounting
    print(json.dumps({'gzip_level': level, 'files': sizes, 'mount_and_styles': mounting}))
