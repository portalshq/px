"""Offline installer checks; the signature verifier is a controlled test double."""
import hashlib
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / 'install.sh'


class SignedInstallTests(unittest.TestCase):
    def test_verified_assets_and_failure_before_replacement(self):
        targets = {('Darwin', 'arm64'): 'aarch64-apple-darwin', ('Darwin', 'x86_64'): 'x86_64-apple-darwin', ('Linux', 'x86_64'): 'x86_64-unknown-linux-gnu', ('Linux', 'aarch64'): 'aarch64-unknown-linux-gnu'}
        target = targets.get((platform.system(), platform.machine()))
        if not target:
            self.skipTest('unsupported test platform')
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            assets, tools, home, install = (root / p for p in ('assets', 'tools', 'home', 'install'))
            for directory in (assets, tools, home, install):
                directory.mkdir()
            for name in (f'px-{target}', f'px-mcp-server-{target}'):
                (assets / name).write_text('#!/bin/sh\nif [ "$1" = plugin ]; then echo "$*" >> "$CALL_LOG"; fi\nif [ "$1 $2" = "plugin info" ]; then if [ -n "${INSTALLED_VERSION:-}" ]; then printf "{\\n  \\"version\\": \\"%s\\"\\n}\\n" "$INSTALLED_VERSION"; exit 0; fi; exit 1; fi\nif [ "$1 $2" = "plugin enable" ] && [ "${FAIL_ENABLE:-0}" = 1 ]; then exit 1; fi\necho fixture\n')
            with zipfile.ZipFile(assets / 'px-skills.zip', 'w') as archive:
                archive.writestr('ask-px/SKILL.md', 'fixture')
            (assets / f'px-verify-optimizer-release-{target}').write_text('#!/bin/sh\n[ "${FAIL_OPTIMIZER:-0}" = 0 ]\n')
            payload = root / 'payload'
            payload.mkdir()
            (payload / 'plugin.yaml').write_text('id: px.optimizer\nversion: 0.1.0\n')
            with tarfile.open(assets / f'px-optimizer-{target}.tar.gz', 'w:gz') as archive:
                archive.add(payload / 'plugin.yaml', arcname='plugin.yaml')
            for name in ('optimizer-release.json', 'manifest.json', 'manifest.sig'):
                (assets / name).write_text('{}')
            names = [f'px-{target}', f'px-mcp-server-{target}', 'px-skills.zip', f'px-optimizer-{target}.tar.gz', f'px-verify-optimizer-release-{target}', 'optimizer-release.json', 'manifest.json', 'manifest.sig']
            (assets / 'SHA256SUMS').write_text(''.join(f'{hashlib.sha256((assets / name).read_bytes()).hexdigest()}  ./{name}\n' for name in names))
            (assets / 'SHA256SUMS.sigstore.json').write_text('{}')
            (tools / 'curl').write_text('#!/bin/sh\nwhile [ "$#" -gt 0 ]; do case "$1" in https://*) url="$1";; --output) shift; out="$1";; esac; shift; done\ncp "$FIXTURE/${url##*/}" "$out"\n')
            (tools / 'cosign').write_text('#!/bin/sh\n[ "${FAIL_SIGNATURE:-0}" = 0 ]\n')
            for tool in tools.iterdir():
                tool.chmod(0o755)
            env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ['PATH'], HOME=str(home), FIXTURE=str(assets), CALL_LOG=str(root / 'calls'), PX_INSTALL_DIR=str(install), PX_INSTALL_BASE_URL='https://fixture.test', PX_TELEMETRY='0')
            def run():
                return subprocess.run(['bash', str(SCRIPT)], env=env, capture_output=True, text=True)
            result = run()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((install / 'px').read_text(), (assets / names[0]).read_text())
            (root / 'calls').write_text('')
            env['INSTALLED_VERSION'] = '0.0.9'
            result = run()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('plugin upgrade ', (root / 'calls').read_text())
            self.assertNotIn('plugin enable', (root / 'calls').read_text())
            del env['INSTALLED_VERSION']
            (install / 'px').write_text('previous installation')
            (home / '.agents/skills/ask-px/SKILL.md').write_text('previous skill')
            env['FAIL_ENABLE'] = '1'
            self.assertNotEqual(run().returncode, 0)
            self.assertEqual((install / 'px').read_text(), 'previous installation')
            self.assertEqual((home / '.agents/skills/ask-px/SKILL.md').read_text(), 'previous skill')
            del env['FAIL_ENABLE']
            env['FAIL_OPTIMIZER'] = '1'
            self.assertNotEqual(run().returncode, 0)
            self.assertEqual((install / 'px').read_text(), 'previous installation')
            del env['FAIL_OPTIMIZER']
            (assets / names[0]).write_text('tampered')
            self.assertNotEqual(run().returncode, 0)
            self.assertEqual((install / 'px').read_text(), 'previous installation')
            env['FAIL_SIGNATURE'] = '1'
            self.assertNotEqual(run().returncode, 0)
            self.assertEqual((install / 'px').read_text(), 'previous installation')


if __name__ == '__main__':
    unittest.main()
