#!/usr/bin/env python3
"""Smoke-test an independently verified optimizer package in an isolated PX home.

Usage: python3 scripts/test-optimizer-package.py ARCHIVE PX_BINARY MCP_BINARY
The caller must verify the archive's release signature and hash first.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    archive_path, px, mcp = (Path(arg).resolve(strict=True) for arg in sys.argv[1:])
    with tempfile.TemporaryDirectory(prefix='px-package-test-') as temp:
        root = Path(temp)
        home, package = root / 'home', root / 'package'
        home.mkdir()
        package.mkdir()
        with tarfile.open(archive_path) as archive:
            # The signed release packager only permits these regular files and
            # directories; enforce the same rule before any local extraction.
            allowed = {'plugin.yaml', 'LICENSE', 'bin', 'skills', 'skills/px-optimize', 'skills/px-optimize/SKILL.md', 'bin/px-plugin-optimizer', 'bin/px-plugin-optimizer.exe'}
            members = archive.getmembers()
            names = [member.name.rstrip('/') for member in members]
            if len(names) != len(set(names)) or any(name not in allowed for name in names) or any(not (m.isfile() or m.isdir()) for m in members):
                raise ValueError('unexpected optimizer package contents')
            archive.extractall(package)
        env = dict(os.environ, HOME=str(home), USERPROFILE=str(home), PX_DIR=str(home / '.px'), PX_CLI_PATH=str(px))
        for args in (['plugin', 'install', str(package)], ['plugin', 'enable', 'px.optimizer']):
            subprocess.run([str(px), *args], env=env, check=True, stdout=subprocess.DEVNULL)
        result = subprocess.run([str(px), 'plugin', 'call', 'px.optimizer', 'adapters', '--input', '{}'], env=env, capture_output=True, text=True, check=True)
        json.loads(result.stdout)
        requests = (
            {'jsonrpc': '2.0', 'id': 1, 'method': 'tools/list', 'params': {}},
            {'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call', 'params': {'name': 'px_optimize_adapters', 'arguments': {}}},
        )
        result = subprocess.run([str(mcp)], env=env, input=''.join(json.dumps(r) + '\n' for r in requests), capture_output=True, text=True, timeout=60, check=True)
        replies = [json.loads(line) for line in result.stdout.splitlines()]
        if len(replies) != len(requests):
            raise RuntimeError('MCP gateway returned an unexpected number of responses')
        for request, reply in zip(requests, replies):
            if 'error' in reply or reply.get('result', {}).get('isError'):
                raise RuntimeError(f'MCP optimizer request failed: {reply}')
            if request['id'] == 1 and 'px_optimize_adapters' not in [tool['name'] for tool in reply['result']['tools']]:
                raise RuntimeError('MCP optimizer tool is missing')
    print('Verified installed optimizer CLI and MCP invocation')


if __name__ == '__main__':
    main()
