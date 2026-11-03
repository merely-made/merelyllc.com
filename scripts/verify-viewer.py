#!/usr/bin/env python3
"""Verify the immutable, browser-qualified viewer and its delivery ceilings."""
import gzip
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
assets = root / 'assets/graphshell'
provenance = json.loads((assets / 'provenance.json').read_text())
assert provenance['runtime_source'] == '14b91e3df2e239924bdb3d7b88a58d00f40871ae'
assert provenance['files']['pkg/graphshell_web_bg.wasm'] == 'cbe148bce26990274974a23bedcc6b32b80611deaa3b4766a2aac6e0568c255e'
assert provenance['files']['pkg/graphshell_web.js'] == 'd05f60d85c8aca2779d181d73df953d8afc0a9cc0eb92709fcb171295f9b7e81'
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
