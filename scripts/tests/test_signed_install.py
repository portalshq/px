"""Offline signed-installer and crash-recovery tests (release verifier is stubbed)."""
import hashlib
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / "install.sh"
FAKE_PX = r'''#!/bin/sh
state="$PLUGIN_STATE"
args="$1 $2"
die_once() {
  if [ "${KILL_AT:-}" = "$1" ] && [ ! -f "$PLUGIN_STATE.killed" ]; then
    : > "$PLUGIN_STATE.killed"
    kill -KILL "$PPID"
    exit 137
  fi
}
if [ "$1" = "--version" ]; then
  echo fixture
  if [ "$(basename "$0")" = "px" ]; then die_once px-version; else die_once mcp-version; fi
fi
if [ "$args" = "plugin info" ]; then
  if [ -f "$state" ]; then
    version="$(sed -n '1p' "$state")"
    printf '{\n  "version": "%s"\n}\n' "$version"
    die_once plugin-info
    exit 0
  fi
  exit 1
fi
if [ "$args" = "plugin install" ]; then
  [ "${FAIL_INSTALL:-0}" = 0 ] || exit 1
  version="$(sed -n 's/^version:[[:space:]]*//p' "$3/plugin.yaml" | tr -d '\"')"
  printf '%s\nfalse\n' "$version" > "$state"
  printf '%s\n' "$*" >> "$CALL_LOG"
  die_once plugin-install
  exit 0
fi
if [ "$args" = "plugin upgrade" ]; then
  [ "${FAIL_UPGRADE:-0}" = 0 ] || exit 1
  old_enabled="$(sed -n '2p' "$state")"
  version="$(sed -n 's/^version:[[:space:]]*//p' "$3/plugin.yaml" | tr -d '\"')"
  printf '%s\n%s\n' "$version" "$old_enabled" > "$state"
  printf '%s\n' "$*" >> "$CALL_LOG"
  die_once plugin-upgrade
  exit 0
fi
if [ "$args" = "plugin enable" ]; then
  [ "${FAIL_ENABLE:-0}" = 0 ] || exit 1
  version="$(sed -n '1p' "$state")"
  printf '%s\ntrue\n' "$version" > "$state"
  printf '%s\n' "$*" >> "$CALL_LOG"
  die_once plugin-enable
  exit 0
fi
if [ "$args" = "plugin disable" ]; then
  version="$(sed -n '1p' "$state")"
  printf '%s\nfalse\n' "$version" > "$state"
  exit 0
fi
exit 0
'''


class SignedInstallTests(unittest.TestCase):
    def setUp(self):
        targets = {
            ("Darwin", "arm64"): "aarch64-apple-darwin",
            ("Darwin", "x86_64"): "x86_64-apple-darwin",
            ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
            ("Linux", "aarch64"): "aarch64-unknown-linux-gnu",
        }
        self.target = targets.get((platform.system(), platform.machine()))
        if not self.target:
            self.skipTest("unsupported test platform")
        self.temporary = tempfile.TemporaryDirectory()
        root = Path(self.temporary.name)
        self.assets, self.tools, self.home, self.install = (root / p for p in ("assets", "tools", "home", "install"))
        for directory in (self.assets, self.tools, self.home, self.install):
            directory.mkdir()
        self.plugin_state = root / "plugin-state"
        self.call_log = root / "calls"
        for name in (f"px-{self.target}", f"px-mcp-server-{self.target}"):
            executable = self.assets / name
            executable.write_text(FAKE_PX)
            executable.chmod(0o755)
        with zipfile.ZipFile(self.assets / "px-skills.zip", "w") as archive:
            archive.writestr("ask-px/SKILL.md", "fixture")
        payload = root / "payload"
        payload.mkdir()
        (payload / "plugin.yaml").write_text("id: px.optimizer\nversion: 0.1.0\n")
        with tarfile.open(self.assets / f"px-optimizer-{self.target}.tar.gz", "w:gz") as archive:
            archive.add(payload / "plugin.yaml", arcname="plugin.yaml")
        verifier = self.assets / f"px-verify-optimizer-release-{self.target}"
        verifier.write_text("#!/bin/sh\n[ \"${FAIL_OPTIMIZER:-0}\" = 0 ]\n")
        verifier.chmod(0o755)
        for name in ("optimizer-release.json", "manifest.json", "manifest.sig"):
            (self.assets / name).write_text("{}")
        names = [
            f"px-{self.target}", f"px-mcp-server-{self.target}", "px-skills.zip",
            f"px-optimizer-{self.target}.tar.gz", f"px-verify-optimizer-release-{self.target}",
            "optimizer-release.json", "manifest.json", "manifest.sig",
        ]
        (self.assets / "SHA256SUMS").write_text("".join(
            f"{hashlib.sha256((self.assets / name).read_bytes()).hexdigest()}  ./{name}\n" for name in names
        ))
        (self.assets / "SHA256SUMS.sigstore.json").write_text("{}")
        curl = self.tools / "curl"
        curl.write_text('#!/bin/sh\nwhile [ "$#" -gt 0 ]; do case "$1" in https://*) url="$1";; --output) shift; out="$1";; esac; shift; done\ncp "$FIXTURE/${url##*/}" "$out"\n')
        cosign = self.tools / "cosign"
        cosign.write_text('#!/bin/sh\n[ "${FAIL_SIGNATURE:-0}" = 0 ]\n')
        for tool in self.tools.iterdir():
            tool.chmod(0o755)
        self.env = dict(
            os.environ,
            PATH=str(self.tools) + os.pathsep + os.environ["PATH"],
            HOME=str(self.home), FIXTURE=str(self.assets), CALL_LOG=str(self.call_log),
            PLUGIN_STATE=str(self.plugin_state), PX_INSTALL_DIR=str(self.install),
            PX_INSTALL_BASE_URL="https://fixture.test", PX_TELEMETRY="0",
        )

    def tearDown(self):
        self.temporary.cleanup()

    def run_installer(self, **env):
        return subprocess.run(["bash", str(SCRIPT)], env=dict(self.env, **env), capture_output=True, text=True)

    def test_install_upgrade_disabled_state_and_preflight_rejection(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.install / "px").read_text(), (self.assets / f"px-{self.target}").read_text())
        self.assertEqual(self.plugin_state.read_text().splitlines(), ["0.1.0", "true"])

        self.plugin_state.write_text("0.0.9\ntrue\n")
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("plugin upgrade", self.call_log.read_text())
        self.assertEqual(self.plugin_state.read_text().splitlines(), ["0.1.0", "true"])

        self.plugin_state.write_text("0.0.9\nfalse\n")
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.plugin_state.read_text().splitlines(), ["0.1.0", "false"])
        self.assertFalse((self.home / ".px/installer-transaction").exists())

        before = (self.install / "px").read_bytes()
        (self.assets / f"px-{self.target}").write_text("tampered")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.install / "px").read_bytes(), before)
        result = self.run_installer(FAIL_SIGNATURE="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.install / "px").read_bytes(), before)

    def test_force_kill_resumes_each_install_phase(self):
        for kill_at in ("px-version", "mcp-version", "plugin-install", "plugin-enable"):
            with self.subTest(kill_at=kill_at):
                # Isolate each independent first install.
                for path in (self.home / ".px", self.home / ".agents", self.install):
                    import shutil
                    shutil.rmtree(path, ignore_errors=True)
                    path.mkdir(parents=True)
                self.plugin_state.unlink(missing_ok=True)
                Path(str(self.plugin_state) + ".killed").unlink(missing_ok=True)
                result = self.run_installer(KILL_AT=kill_at)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                txn = self.home / ".px/installer-transaction"
                self.assertTrue((txn / "STATE").is_file(), f"transaction missing after {kill_at}: {result.stdout}{result.stderr}")
                result = self.run_installer()
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.plugin_state.read_text().splitlines(), ["0.1.0", "true"])
                self.assertTrue((self.home / ".agents/skills/ask-px/SKILL.md").is_file())
                self.assertFalse(txn.exists())

    def test_failure_keeps_recoverable_verified_transaction(self):
        result = self.run_installer(FAIL_ENABLE="1")
        self.assertNotEqual(result.returncode, 0)
        txn = self.home / ".px/installer-transaction"
        self.assertTrue((txn / "STATE").is_file())
        self.assertEqual(self.plugin_state.read_text().splitlines(), ["0.1.0", "false"])
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.plugin_state.read_text().splitlines(), ["0.1.0", "true"])
        self.assertFalse(txn.exists())


if __name__ == "__main__":
    unittest.main()
