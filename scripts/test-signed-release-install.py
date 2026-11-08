#!/usr/bin/env python3
"""Exercise a signed Linux release bundle without publishing it.

Run in the cli-release workflow after signing SHA256SUMS. The bundle must have
been signed by that workflow on the tag in RELEASE_TAG; no verifier is mocked.
"""
import functools
import hashlib
import http.server
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import threading


def main():
    if len(sys.argv) != 2 or not os.environ.get('RELEASE_TAG'):
        raise SystemExit('usage: RELEASE_TAG=vX.Y.Z python scripts/test-signed-release-install.py ARTIFACTS')
    source = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix='px-release-e2e-') as temporary:
        root = Path(temporary)
        assets, home, install = (root / name for name in ('assets', 'home', 'bin'))
        shutil.copytree(source, assets)
        home.mkdir()
        install.mkdir()
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(assets)))
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            env = dict(os.environ, HOME=str(home), PX_DIR=str(home / '.px'), PX_INSTALL_DIR=str(install), PX_INSTALL_BASE_URL=f'http://127.0.0.1:{server.server_port}', PX_TELEMETRY='0', VERSION=os.environ['RELEASE_TAG'])
            installer = Path(__file__).resolve().parent / 'install.sh'
            def install_once():
                return subprocess.run(['bash', str(installer)], env=env, capture_output=True, text=True, timeout=180)
            result = install_once()
            if result.returncode:
                raise RuntimeError('signed installer failed:\n' + result.stdout + result.stderr)
            px = install / 'px'
            mcp = install / 'px-mcp-server'
            registry = json.loads(subprocess.check_output([str(px), 'plugin', 'list'], env=env, text=True))
            if not registry['installed']['px.optimizer']['enabled']:
                raise RuntimeError('fresh installation did not enable the optimizer')
            result = subprocess.run([str(mcp)], input=json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call', 'params': {'name': 'px_optimize_adapters', 'arguments': {}}}) + '\n', env=dict(env, PX_CLI_PATH=str(px)), capture_output=True, text=True, check=True, timeout=60)
            response = json.loads(result.stdout)
            if 'error' in response or response.get('result', {}).get('isError'):
                raise RuntimeError(f'installed MCP optimizer call failed: {response}')
            subprocess.run([str(px), 'plugin', 'disable', 'px.optimizer'], env=env, check=True, stdout=subprocess.DEVNULL)
            result = install_once()
            if result.returncode:
                raise RuntimeError('signed reinstall failed:\n' + result.stdout + result.stderr)
            registry = json.loads(subprocess.check_output([str(px), 'plugin', 'list'], env=env, text=True))
            if registry['installed']['px.optimizer']['enabled']:
                raise RuntimeError('reinstall re-enabled the explicitly disabled optimizer')
            before = hashlib.sha256(px.read_bytes()).digest()
            targets = {
                ('Darwin', 'arm64'): 'aarch64-apple-darwin',
                ('Darwin', 'x86_64'): 'x86_64-apple-darwin',
                ('Linux', 'aarch64'): 'aarch64-unknown-linux-gnu',
                ('Linux', 'x86_64'): 'x86_64-unknown-linux-gnu',
            }
            target = targets.get((platform.system(), platform.machine()))
            if target is None:
                raise RuntimeError('unsupported release-test platform')
            package = assets / f'px-optimizer-{target}.tar.gz'
            with package.open('ab') as handle:
                handle.write(b'tampered')
            if install_once().returncode == 0 or hashlib.sha256(px.read_bytes()).digest() != before:
                raise RuntimeError('tampered release was not rejected before installation')
        finally:
            server.shutdown()
            server.server_close()
            thread.join()
    print('Signed release installation, MCP, disabled-state preservation, and tampering checks passed')


if __name__ == '__main__':
    main()
