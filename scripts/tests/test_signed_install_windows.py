"""Windows installer integration with local assets and controlled executables."""
import functools
import hashlib
import http.server
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import threading
import unittest
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / 'install.ps1'
TARGET = 'x86_64-pc-windows-msvc'


@unittest.skipUnless(os.name == 'nt', 'requires Windows PowerShell')
class WindowsInstallTests(unittest.TestCase):
    def test_install_and_restore_on_optimizer_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            assets, tools, home, install = (root / p for p in ('assets', 'tools', 'home', 'install'))
            for directory in (assets, tools, home, install):
                directory.mkdir()
            source = root / 'fixture.rs'
            source.write_text(r'''fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("plugin") && args.get(2).map(String::as_str) == Some("info") { std::process::exit(1); }
    if args.get(1).map(String::as_str) == Some("plugin") && args.get(2).map(String::as_str) == Some("enable") && std::env::var("FAIL_ENABLE").as_deref() == Ok("1") { std::process::exit(1); }
    if args.get(1).map(String::as_str) == Some("verify-blob") && std::env::var("FAIL_SIGNATURE").as_deref() == Ok("1") { std::process::exit(1); }
    println!("fixture");
}''')
            exe = root / 'fixture.exe'
            subprocess.run(['rustc', str(source), '-o', str(exe)], check=True)
            shutil.copyfile(exe, tools / 'cosign.exe')
            names = [f'px-{TARGET}.exe', f'px-mcp-server-{TARGET}.exe', f'px-verify-optimizer-release-{TARGET}.exe']
            for name in names:
                shutil.copyfile(exe, assets / name)
            with zipfile.ZipFile(assets / 'px-skills.zip', 'w') as archive:
                archive.writestr('ask-px/SKILL.md', 'fixture')
            payload = root / 'plugin.yaml'
            payload.write_text('id: px.optimizer\nversion: 0.1.0\n')
            with tarfile.open(assets / f'px-optimizer-{TARGET}.tar.gz', 'w:gz') as archive:
                archive.add(payload, arcname='plugin.yaml')
            names += ['px-skills.zip', f'px-optimizer-{TARGET}.tar.gz', 'optimizer-release.json', 'manifest.json', 'manifest.sig']
            for name in names[-3:]:
                (assets / name).write_text('{}')
            (assets / 'SHA256SUMS').write_text(''.join(f'{hashlib.sha256((assets / name).read_bytes()).hexdigest()}  ./{name}\n' for name in names))
            (assets / 'SHA256SUMS.sigstore.json').write_text('{}')
            handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(assets))
            server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                env = dict(os.environ, USERPROFILE=str(home), TEMP=str(root), PATH=str(tools) + os.pathsep + os.environ['PATH'], PX_TELEMETRY='0')
                command = ['pwsh', '-NoProfile', '-File', str(SCRIPT), '-InstallDir', str(install), '-BaseUrl', f'http://127.0.0.1:{server.server_port}']
                def run():
                    return subprocess.run(command, env=env, capture_output=True, text=True)
                result = run()
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                (install / 'px.exe').write_bytes(b'previous')
                (home / '.agents/skills/ask-px/SKILL.md').write_text('previous skill')
                env['FAIL_ENABLE'] = '1'
                self.assertNotEqual(run().returncode, 0)
                self.assertEqual((install / 'px.exe').read_bytes(), b'previous')
                self.assertEqual((home / '.agents/skills/ask-px/SKILL.md').read_text(), 'previous skill')
                del env['FAIL_ENABLE']
                env['FAIL_SIGNATURE'] = '1'
                self.assertNotEqual(run().returncode, 0)
                self.assertEqual((install / 'px.exe').read_bytes(), b'previous')
            finally:
                server.shutdown()
                server.server_close()
                thread.join()


if __name__ == '__main__':
    unittest.main()
