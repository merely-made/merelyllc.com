"""Run the combined cone's bounded review receipts, one headed page at a time."""
import argparse
import json
import pathlib
import re
import subprocess
import sys


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--root', type=pathlib.Path, required=True)
parser.add_argument('--sink', type=pathlib.Path, required=True)
parser.add_argument('--bundle', choices=['viewer', 'default', 'viewer-release'], required=True)
parser.add_argument('--port', type=int, default=8982, help='First receipt port; occupied ports are refused.')
parser.add_argument('--only', help='Run comma-separated named rows.')
parser.add_argument('--suffix', default='', help='New output suffix for a replacement control.')
args = parser.parse_args()
if args.suffix and not re.fullmatch('[a-z0-9-]+', args.suffix):
    parser.error('suffix must contain only lowercase letters, digits or hyphens')
root = args.root.resolve()
speed = 'nodes=300&seed=7&links=none&gpu=off'
fifty = 'nodes=24&seed=7&links=none&gpu=off&physics_speed=1'
rows = [
    ('slow', 'p6_tree_speed_slow', 'physics_speed=0.2&gpu=off', 'pass', ''),
    ('fast', 'p6_tree_speed_fast', speed + '&physics_speed=1', 'pass', ''),
    ('fifty', 'p6_tree_speed_fast_control', fifty, 'pass', ''),
    ('stall', 'p6_tree_speed_fast_planted', speed + '&physics_speed=max&physics_plant_stall_ms=10', 'pass', ''),
    ('slow-max', 'p6_tree_speed_fast_slow_max', speed + '&physics_speed=1&physics_plant_max_frame_ms=3000', 'pass', ''),
    ('cap', 'p6_tree_speed_fast_control_capped', fifty + '&physics_plant_owed=1x', 'pass', ''),
    ('select', 'p6_tree_speed_select', speed, 'pass', ''),
    ('reader', 'p4_tree_canvas_reader', 'gpu=off', 'pass', ''),
    ('reader-control', 'p4_tree_canvas_reader', 'gpu=off&plant_a11y=missing_action', 'fail', 'reader-buttons'),
    ('roles', 'p4_tree_role_controls', 'gpu=off', 'pass', ''),
    ('arrangement', 'p4_tree_arrangement_roles', 'gpu=off', 'pass', ''),
    ('framing', 'p4_tree_physics_framing_control', 'gpu=off', 'pass', ''),
    ('page-error', 'p4_tree_physics_framing_control', 'gpu=off&plant_page_error=throw', 'gate', 'receipt gate'),
    ('worker', 'p0_worker_worker', 'gpu=off', 'pass', ''),
    ('worker-off', 'p0_worker_main', 'gpu=off&physics_period_source=main', 'pass', ''),
    ('worker-failed', 'p0_worker_failed', 'gpu=off&physics_plant_worker=fail', 'pass', ''),
]
if args.bundle == 'viewer':
    rows += [
        ('remote-refused', 'p4_tree_remote_absent', 'gpu=off', 'fail', 'Remote session'),
        ('saved-refused', 'p4_tree_saved_edit', 'gpu=off&app=local', 'fail', 'source'),
        ('remote-export', 'p4_tree_canvas_reader', 'gpu=off&signal=ws://127.0.0.1:8999', 'pass', ''),
    ]
elif args.bundle == 'default':
    rows += [('main-select', 'physics_speed_select', speed, 'pass', '')]

if args.bundle == 'viewer-release':
    rows = [('site21', 'p0_viewer_site_frames_21', 'nodes=21&seed=7&links=none&physics_speed=1&gpu=off', 'pass', '')]
if args.only:
    names = set(args.only.split(','))
    unknown = names - {row[0] for row in rows}
    if unknown:
        parser.error('unknown rows: ' + ', '.join(sorted(unknown)))
    rows = [row for row in rows if row[0] in names]
run_name = args.bundle + ('-' + args.suffix if args.suffix else '')
results = []
for index, (label, scenario, query, expected, reason) in enumerate(rows):
    out = root / 'receipts' / (run_name + '-' + label)
    assert not out.exists(), out
    page = 'index.html' if label == 'main-select' else 'tree.html'
    command = [
        sys.executable, str(root / 'run-linux-receipt.py'),
        '--web', str(root / args.bundle), '--out', str(out),
        '--sink', str(args.sink), '--scenario', scenario, '--page', page,
        '--query', query, '--port', str(args.port + index), '--timeout', '360',
        '--unsafe-webgpu', '--chrome-arg=--enable-features=Vulkan',
        '--chrome-arg=--use-angle=vulkan',
    ]
    run = subprocess.run(command, capture_output=True, text=True)
    (root / (run_name + '-' + label + '-run.log')).write_text(run.stdout + run.stderr)
    row = {'label': label, 'scenario': scenario, 'query': query, 'expected': expected,
           'reason': reason, 'harness_exit': run.returncode, 'receipt': out.name, 'accepted': False}
    if (out / 'environment.json').exists():
        environment = json.loads((out / 'environment.json').read_text())
        row.update({key: environment.get(key) for key in
                    ['result', 'bundle_sha256', 'scenario_sha256', 'cpu_before_percent',
                     'page_errors', 'gate_failures', 'elapsed_seconds']})
        text = (out / 'scenario.done').read_text() if (out / 'scenario.done').exists() else ''
        clean = not row['page_errors'] and not row['gate_failures']
        if expected == 'pass':
            row['accepted'] = run.returncode == 0 and row['result'] == 'ok' and clean
        elif expected == 'fail':
            row['accepted'] = run.returncode == 0 and row['result'] == 'fail' and clean and reason in text
        else:
            row['accepted'] = (run.returncode == 0 and row['result'] == 'fail'
                               and bool(row['gate_failures']) and reason in text)
    results.append(row)
    (root / (run_name + '-review-summary.json')).write_text(json.dumps(results, indent=2) + '\n')
    print(args.bundle, label, row.get('result'), 'accepted' if row['accepted'] else 'MISS', flush=True)

sys.exit(0 if all(row['accepted'] for row in results) else 1)
