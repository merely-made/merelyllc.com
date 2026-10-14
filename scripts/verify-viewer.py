#!/usr/bin/env python3
"""Verify the immutable, browser-qualified viewer and its delivery ceilings."""
import gzip
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
assets = root / 'assets/graphshell'
provenance = json.loads((assets / 'provenance.json').read_text())
assert provenance['runtime_source'] == '13ede033f8afe6af58db81921b3ded61108602b9'
assert provenance['files']['pkg/graphshell_web_bg.wasm'] == 'a7900793a74eb7a0c734120f0ed15f91e0e1df0416f437216e40db81e35c3ce4'
assert provenance['files']['pkg/graphshell_web.js'] == '3f075065455888c46d8e88d5cb18dc345220f5b9846904dce6623422af576f25'
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
