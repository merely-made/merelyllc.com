#!/usr/bin/env python3
"""Verify the immutable, browser-qualified viewer and its delivery ceilings."""
import gzip
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
assets = root / 'assets/graphshell'
provenance = json.loads((assets / 'provenance.json').read_text())
assert provenance['runtime_source'] == '377a0e30c84e2059a945583833008e84ae045fce'
assert provenance['files']['pkg/graphshell_web_bg.wasm'] == '885f9ce1a863afb7b1d2de04ac7d81ccd4a517e51d000b9bb4c1da9ab1d18cd3'
assert provenance['files']['pkg/graphshell_web.js'] == '04273e52f64ca2665a06c1d8e60475d7082b4b2d15aa44bcfc8f1d20ee0730f8'
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
