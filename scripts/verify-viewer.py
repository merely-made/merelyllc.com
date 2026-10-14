#!/usr/bin/env python3
"""Verify the immutable, browser-qualified viewer and its delivery ceilings."""
import gzip
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
assets = root / 'assets/graphshell'
provenance = json.loads((assets / 'provenance.json').read_text())
assert provenance['runtime_source'] == '56859e30292405e682f28f3841b76e4f141acf73'
assert provenance['files']['pkg/graphshell_web_bg.wasm'] == 'a6e094bca4e274bb9476e00051891178a7ec8509ef2ee71ddf883ea2e812215e'
assert provenance['files']['pkg/graphshell_web.js'] == '157c5c2574754fa552abe5930a7920143578ceba0c3d010936b8e008cb530c5a'
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
