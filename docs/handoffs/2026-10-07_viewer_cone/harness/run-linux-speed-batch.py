"""Rerun the handoff's five outstanding speed receipts, sequentially.

Uses the transferred dev bundles, or release bundles with --release. Output
directories must be new. Adapter flags match the separately recorded AMD
hardware-adapter probe. Results are in each receipt's environment.json;
scenario assertion failures are recorded independently of harness errors.
"""
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent
ROWS = [
    ('default', 'p6_tree_speed_fast', 'nodes=300&seed=7&links=none&gpu=off&physics_speed=max'),
    ('default', 'p6_tree_speed_fast_control', 'nodes=24&seed=7&links=none&gpu=off&physics_speed=50'),
    ('viewer', 'p6_tree_speed_fast', 'nodes=300&seed=7&links=none&gpu=off&physics_speed=max'),
    ('viewer', 'p6_tree_speed_fast_control', 'nodes=24&seed=7&links=none&gpu=off&physics_speed=50'),
    ('viewer', 'p6_tree_speed_slow', 'physics_speed=0.2'),
]

for index, (bundle, scenario, query) in enumerate(ROWS):
    # --skip-default-fast reuses the already recorded standalone attempt.
    if '--skip-default-fast' in sys.argv and index == 0:
        continue
    if '--skip-default-control' in sys.argv and index == 1:
        continue
    if '--release' in sys.argv:
        bundle += '-release'
    output = ROOT / 'receipts' / (bundle + '-' + scenario + '-hardware')
    print('RUN', bundle, scenario, flush=True)
    result = subprocess.run([
        sys.executable, str(ROOT / 'run-linux-receipt.py'),
        '--web', str(ROOT / bundle), '--out', str(output),
        '--sink', str(ROOT / 'graphshell-web-sink.py'), '--scenario', scenario,
        '--query', query, '--port', str(8960 + index), '--timeout', '240',
        '--unsafe-webgpu', '--chrome-arg=--enable-features=Vulkan',
        '--chrome-arg=--use-angle=vulkan',
    ])
    if result.returncode:
        print('HARNESS ERROR', result.returncode, bundle, scenario, flush=True)
