#!/usr/bin/env python3
"""Run the local mock-OAuth browser suite with ownership-aware teardown.

Only the browser's PNG captures and result.json belong in CI artifacts. Runtime
state and startup logs stay private and are never printed or published.
"""
import argparse
import importlib.util
import os
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('host_dev', ROOT / 'scripts/host-dev.py')
host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(host)


def run(args):
    output = args.output.resolve()
    if output == ROOT or ROOT in output.parents:
        raise ValueError('Choose an artifact directory outside the source checkout')
    # A unique instance prevents a failed startup from claiming another run's data.
    instance = 'browser-ci-' + secrets.token_hex(8)
    state = host.state_path(instance)
    if state.exists():
        raise ValueError('Generated instance already exists; preserve its resources')
    helper = [sys.executable, str(ROOT / 'scripts/host-dev.py')]
    start = helper + ['up', instance]
    if args.managed:
        start.append('--managed')
    if args.dx:
        start.extend(['--dx', args.dx])
    smoke = [sys.executable, str(ROOT / 'scripts/browser-smoke.py'), '--host', '--instance', instance, '--showcase', '--output', str(output)]
    if args.chromium:
        smoke.extend(['--chromium', args.chromium])
    code = 0
    with tempfile.TemporaryDirectory(prefix='app-browser-ci-private-') as private:
        try:
            with (Path(private) / 'startup.log').open('w') as log:
                os.chmod(log.name, 0o600)
                subprocess.run(start, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
            print('Disposable local app ready; running synthetic browser checks.', flush=True)
            subprocess.run(smoke, cwd=ROOT, check=True)
        except subprocess.CalledProcessError as error:
            code = error.returncode or 1
            print('Browser CI failed; private runtime logs were not published.', file=sys.stderr)
        finally:
            if state.exists():
                # host-dev verifies the worktree, process identity and resource labels.
                cleanup = subprocess.run(helper + ['down', instance], cwd=ROOT)
                if cleanup.returncode:
                    code = code or cleanup.returncode
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--dx')
    parser.add_argument('--chromium')
    parser.add_argument('--managed', action='store_true')
    args = parser.parse_args()
    try:
        return run(args)
    except (ValueError, OSError) as error:
        print('Browser CI setup failed: ' + type(error).__name__, file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
