"""Run one headed receipt using an existing bundle and an isolated Chrome.

No builds or browser input. Outputs and profile must be new. The sink and
Chrome process group belong to this run; unrelated processes are untouched.
Linux /proc/stat samples record total CPU use, including the receipt itself.
"""

import argparse
import hashlib
import json
import os
import pathlib
import platform
import signal
import socket
import subprocess
import time
import urllib.request


def cpu_ticks():
    ticks = list(map(int, pathlib.Path('/proc/stat').read_text().splitlines()[0].split()[1:9]))
    return sum(ticks), ticks[3] + ticks[4]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--web', required=True, type=pathlib.Path)
    parser.add_argument('--out', required=True, type=pathlib.Path)
    parser.add_argument('--sink', required=True, type=pathlib.Path)
    parser.add_argument('--scenario', required=True)
    parser.add_argument('--page', default='tree.html')
    parser.add_argument('--query', required=True)
    parser.add_argument('--port', type=int, default=8960)
    parser.add_argument('--timeout', type=int, default=240)
    parser.add_argument('--unsafe-webgpu', action='store_true')
    parser.add_argument('--chrome-arg', action='append', default=[])
    args = parser.parse_args()
    args.web = args.web.resolve()
    args.out = args.out.resolve()
    with socket.socket() as port_check:
        port_check.bind(('127.0.0.1', args.port))
    args.out.mkdir(parents=True, exist_ok=False)
    wasm = args.web / 'pkg/graphshell_web_bg.wasm'
    metadata = {
        'machine': platform.node(), 'platform': platform.platform(),
        'bundle_sha256': hashlib.sha256(wasm.read_bytes()).hexdigest(),
        'bundle_bytes': wasm.stat().st_size, 'scenario': args.scenario,
        'scenario_sha256': hashlib.sha256((args.web / 'scenarios' / (args.scenario + '.scn')).read_bytes()).hexdigest(),
        'query': args.query, 'load_before': os.getloadavg(),
        'unsafe_webgpu': args.unsafe_webgpu, 'cpu_samples': [],
    }
    before = cpu_ticks()
    time.sleep(1)
    after = cpu_ticks()
    metadata['cpu_before_percent'] = 100 * (1 - (after[1] - before[1]) / max(1, after[0] - before[0]))
    env = dict(os.environ)
    env.setdefault('XDG_RUNTIME_DIR', '/run/user/' + str(os.getuid()))
    env.setdefault('WAYLAND_DISPLAY', 'wayland-0')
    env.setdefault('DBUS_SESSION_BUS_ADDRESS', 'unix:path=' + env['XDG_RUNTIME_DIR'] + '/bus')
    chrome = None
    sink = None
    started = time.monotonic()
    try:
        with (args.out / 'sink.log').open('w') as sink_log, (args.out / 'chrome.log').open('w') as chrome_log:
            sink = subprocess.Popen(['python3', str(args.sink), '--web', str(args.web), '--out', str(args.out), '--port', str(args.port)], stdout=sink_log, stderr=sink_log, start_new_session=True)
            deadline = time.monotonic() + 10
            while True:
                if sink.poll() is not None:
                    raise RuntimeError('sink exited; see sink.log')
                try:
                    with urllib.request.urlopen('http://127.0.0.1:' + str(args.port) + '/tree.html', timeout=1) as response:
                        if response.status == 200:
                            break
                except OSError:
                    if time.monotonic() > deadline:
                        raise RuntimeError('sink did not start')
                    time.sleep(.25)
            url = ('http://127.0.0.1:' + str(args.port) + '/' + args.page + '?scenario=scenarios/' + args.scenario + '.scn&sink=http://127.0.0.1:' + str(args.port) + '/scenario-receipt&' + args.query)
            command = ['flatpak', 'run', '--filesystem=' + str(args.out), 'com.google.Chrome', '--user-data-dir=' + str(args.out / 'chrome-profile'), '--no-first-run', '--no-default-browser-check', '--enable-logging=stderr', '--ozone-platform=wayland', '--disable-features=CalculateNativeWinOcclusion', '--disable-backgrounding-occluded-windows', '--disable-renderer-backgrounding', '--new-window', '--window-size=1400,900']
            if args.unsafe_webgpu:
                command.append('--enable-unsafe-webgpu')
            command.extend(args.chrome_arg)
            command.append(url)
            metadata['command'] = command
            chrome = subprocess.Popen(command, stdout=chrome_log, stderr=chrome_log, env=env, start_new_session=True)
            previous = cpu_ticks()
            while not (args.out / 'scenario.done').exists():
                if chrome.poll() is not None:
                    raise RuntimeError('Chrome exited; see chrome.log')
                if time.monotonic() - started > args.timeout:
                    raise RuntimeError('receipt timed out')
                time.sleep(1)
                current = cpu_ticks()
                metadata['cpu_samples'].append({'seconds': round(time.monotonic() - started, 2), 'percent': round(100 * (1 - (current[1] - previous[1]) / max(1, current[0] - previous[0])), 2)})
                previous = current
            result = json.loads((args.out / 'result.json').read_text())
            scenario = result['scenario']
            metadata['result'] = scenario['result']['result']
            metadata['page_errors'] = scenario.get('errors', [])
            metadata['gate_failures'] = scenario.get('gate_failures', [])
            print((args.out / 'scenario.done').read_text(), flush=True)
    finally:
        for process in (chrome, sink):
            if process is not None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                    process.wait(timeout=5)
                except ProcessLookupError:
                    pass
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
        metadata['elapsed_seconds'] = round(time.monotonic() - started, 2)
        metadata['load_after'] = os.getloadavg()
        (args.out / 'environment.json').write_text(json.dumps(metadata, indent=2) + '\n')


if __name__ == '__main__':
    main()
