#!/usr/bin/env python3
"""Serve historical regression data and source-bound current page snapshots."""
import hashlib
import json
from pathlib import Path
import re
import shutil

out = Path('.tmp/viewer-linux')
out.mkdir(parents=True, exist_ok=True)
for fixture, destination, receipt in [
    ('viewer-linux', 'viewer-linux-site', 'source-binding.json'),
    ('viewer-site-20261027', 'viewer-current-site', 'product-source-binding.json'),
]:
    source = Path('tests/fixtures') / fixture
    root = Path('.tmp') / destination
    (root / 'repos').mkdir(parents=True, exist_ok=True)
    (root / 'projects/org-profile').mkdir(parents=True, exist_ok=True)
    binding = {'fixture': fixture}
    if (source / 'source.json').exists():
        binding.update(json.loads((source / 'source.json').read_text()))
        for name, expected in binding['files'].items():
            assert hashlib.sha256((source / name).read_bytes()).hexdigest() == expected, name
    for name, target in [('repos.html', 'repos/index.html'),
                         ('org-profile.html', 'projects/org-profile/index.html'),
                         ('history.json', 'repository-host-history.json')]:
        shutil.copyfile(source / name, root / target)
    for name in ['site.css', 'graph-sandbox-mount.js', 'favicon.svg']:
        shutil.copyfile(Path('assets') / name, root / name)
    prefix = re.search(r'data-sandbox-runtime="/(graphshell/[a-f0-9]+)/viewer.js"',
                       (source / 'repos.html').read_text()).group(1)
    shutil.copytree('assets/graphshell', root / prefix, dirs_exist_ok=True)
    binding['runtime'] = json.loads(Path('assets/graphshell/provenance.json').read_text())
    binding['served_inputs'] = {
        str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in root.rglob('*') if p.is_file()
    }
    (out / receipt).write_text(json.dumps(binding, indent=2) + '\n')
