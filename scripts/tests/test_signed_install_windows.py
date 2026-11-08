"""Windows installer signature, upgrade, and interrupted-transaction tests."""
import hashlib
import http.server
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import threading
import time
import unittest
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / "install.ps1"
TARGET = "x86_64-pc-windows-msvc"


@unittest.skipUnless(os.name == "nt", "requires Windows PowerShell")
class WindowsInstallTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.assets, self.tools, self.home, self.install = (root / p for p in ("assets", "tools", "home", "install"))
        for directory in (self.assets, self.tools, self.home, self.install):
            directory.mkdir()
        self.state = root / "plugin-state"
        self.kill_marker = root / "plugin-state.killed"
        self.source = root / "fixture.rs"
        self.source.write_text(r'''
use std::{env, fs, process::Command, thread, time::Duration};
fn interrupt(phase: &str) {
    if env::var("KILL_AT").as_deref() == Ok(phase) {
        let marker = env::var("KILL_MARKER").unwrap();
        if !std::path::Path::new(&marker).exists() {
            fs::write(&marker, "done").unwrap();
            let pid = env::var("INSTALLER_PID").unwrap();
            let _ = Command::new("taskkill").args(["/F", "/PID", &pid]).status();
            thread::sleep(Duration::from_millis(500));
        }
    }
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let a = args.get(1).map(String::as_str).unwrap_or("");
    let b = args.get(2).map(String::as_str).unwrap_or("");
    let state = env::var("PLUGIN_STATE").unwrap();
    if a == "verify-blob" && env::var("FAIL_SIGNATURE").as_deref() == Ok("1") { std::process::exit(1); }
    if a == "--version" {
        println!("fixture");
        if env::current_exe().unwrap().file_name().unwrap().to_string_lossy().starts_with("px-mcp") { interrupt("mcp-version"); }
        else { interrupt("px-version"); }
        return;
    }
    if a == "plugin" && b == "info" {
        if let Ok(contents) = fs::read_to_string(&state) {
            println!("{{\"version\": \"{}\"}}", contents.lines().next().unwrap_or(""));
            return;
        }
        std::process::exit(1);
    }
    if a == "plugin" && (b == "install" || b == "upgrade") {
        let fail = if b == "install" { "FAIL_INSTALL" } else { "FAIL_UPGRADE" };
        if env::var(fail).as_deref() == Ok("1") { std::process::exit(1); }
        let package = args.get(3).unwrap();
        let manifest = fs::read_to_string(format!("{package}/plugin.yaml")).unwrap();
        let version = manifest.lines().find_map(|line| line.strip_prefix("version: ")).unwrap();
        let enabled = if b == "upgrade" {
            fs::read_to_string(&state).unwrap().lines().nth(1).unwrap_or("false").to_owned()
        } else { "false".to_owned() };
        fs::write(&state, format!("{version}\n{enabled}\n")).unwrap();
        interrupt(if b == "install" { "plugin-install" } else { "plugin-upgrade" });
        return;
    }
    if a == "plugin" && b == "enable" {
        if env::var("FAIL_ENABLE").as_deref() == Ok("1") { std::process::exit(1); }
        let version = fs::read_to_string(&state).unwrap().lines().next().unwrap().to_owned();
        fs::write(&state, format!("{version}\ntrue\n")).unwrap();
        interrupt("plugin-enable");
        return;
    }
    println!("fixture");
}
''')
        exe = root / "fixture.exe"
        subprocess.run(["rustc", str(self.source), "-o", str(exe)], check=True)
        shutil.copyfile(exe, self.tools / "cosign.exe")
        for name in (f"px-{TARGET}.exe", f"px-mcp-server-{TARGET}.exe", f"px-verify-optimizer-release-{TARGET}.exe"):
            shutil.copyfile(exe, self.assets / name)
        with zipfile.ZipFile(self.assets / "px-skills.zip", "w") as archive:
            archive.writestr("ask-px/SKILL.md", "fixture")
        plugin = root / "plugin.yaml"
        plugin.write_text("id: px.optimizer\nversion: 0.1.0\n")
        with tarfile.open(self.assets / f"px-optimizer-{TARGET}.tar.gz", "w:gz") as archive:
            archive.add(plugin, arcname="plugin.yaml")
        files = [f"px-{TARGET}.exe", f"px-mcp-server-{TARGET}.exe", "px-skills.zip",
                 f"px-optimizer-{TARGET}.tar.gz", f"px-verify-optimizer-release-{TARGET}.exe"]
        for name in ("optimizer-release.json", "manifest.json", "manifest.sig"):
            (self.assets / name).write_text("{}")
            files.append(name)
        (self.assets / "SHA256SUMS").write_text("".join(
            f"{hashlib.sha256((self.assets / name).read_bytes()).hexdigest()}  ./{name}\n" for name in files
        ))
        (self.assets / "SHA256SUMS.sigstore.json").write_text("{}")
        server = http.server.ThreadingHTTPServer(
            ("127.0.0.1", 0),
            __import__("functools").partial(http.server.SimpleHTTPRequestHandler, directory=str(self.assets)),
        )
        self.server = server
        self.thread = threading.Thread(target=server.serve_forever, daemon=True)
        self.thread.start()
        self.env = dict(os.environ, USERPROFILE=str(self.home), TEMP=str(root),
                        LOCALAPPDATA=str(root / "local"), PATH=str(self.tools) + os.pathsep + os.environ["PATH"],
                        PX_TELEMETRY="0", PLUGIN_STATE=str(self.state), KILL_MARKER=str(self.kill_marker))
        self.command = ["pwsh", "-NoProfile", "-File", str(SCRIPT), "-InstallDir", str(self.install),
                        "-BaseUrl", f"http://127.0.0.1:{server.server_port}"]

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()
        self.tmp.cleanup()

    def run_installer(self, **extra):
        return subprocess.run(self.command, env=dict(self.env, **extra), capture_output=True, text=True, timeout=180)

    def test_signed_install_upgrade_failure_resumes_and_tampering_is_rejected(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.state.read_text().splitlines(), ["0.1.0", "true"])

        self.state.write_text("0.0.9\nfalse\n")
        result = self.run_installer(FAIL_UPGRADE="1")
        self.assertNotEqual(result.returncode, 0)
        transaction = self.home / ".px/installer-transaction"
        self.assertTrue((transaction / "STATE").is_file())
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.state.read_text().splitlines(), ["0.1.0", "false"])
        self.assertFalse(transaction.exists())

        before = (self.install / "px.exe").read_bytes()
        with (self.assets / f"px-{TARGET}.exe").open("ab") as handle:
            handle.write(b"tampered")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.install / "px.exe").read_bytes(), before)
        result = self.run_installer(FAIL_SIGNATURE="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.install / "px.exe").read_bytes(), before)

    def test_force_kill_is_resumed_from_signed_payload(self):
        for phase in ("px-version", "mcp-version", "plugin-install", "plugin-enable"):
            with self.subTest(phase=phase):
                for path in (self.home / ".px", self.home / ".agents", self.install):
                    shutil.rmtree(path, ignore_errors=True)
                    path.mkdir(parents=True)
                self.state.unlink(missing_ok=True)
                self.kill_marker.unlink(missing_ok=True)
                process_env = dict(self.env, KILL_AT=phase)
                process = subprocess.Popen(
                    ["pwsh", "-NoProfile", "-Command",
                     f"$env:INSTALLER_PID=$PID; & '{SCRIPT}' -InstallDir '{self.install}' -BaseUrl 'http://127.0.0.1:{self.server.server_port}'"],
                    env=process_env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                )
                try:
                    stdout, stderr = process.communicate(timeout=120)
                except subprocess.TimeoutExpired:
                    subprocess.run(["taskkill", "/F", "/T", "/PID", str(process.pid)], capture_output=True)
                    self.fail(f"installer did not stop at injected interruption {phase}")
                self.assertNotEqual(process.returncode, 0, stdout + stderr)
                transaction = self.home / ".px/installer-transaction"
                self.assertTrue((transaction / "STATE").is_file(), f"transaction missing after {phase}: {stdout}{stderr}")
                result = self.run_installer()
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.state.read_text().splitlines(), ["0.1.0", "true"])
                self.assertTrue((self.home / ".agents/skills/ask-px/SKILL.md").is_file())
                self.assertFalse(transaction.exists())


if __name__ == "__main__":
    unittest.main()
