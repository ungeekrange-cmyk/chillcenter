#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import pathlib
import stat
import subprocess
import tarfile
import tempfile
import unittest


REPOSITORY = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPOSITORY / "scripts/package-linux.sh"
HELPER = REPOSITORY / "scripts/helpers/linux_package.py"


def elf(machine: int, elf_class: int = 2) -> bytes:
    header = bytearray(64)
    header[:6] = b"\x7fELF" + bytes((elf_class, 1))
    header[18:20] = machine.to_bytes(2, "little")
    return bytes(header)


class LinuxPackageTests(unittest.TestCase):
    def test_service_and_example_keep_state_local_and_nas_read_only(self) -> None:
        config = json.loads((REPOSITORY / "deploy/linux/server.json.example").read_text())
        self.assertEqual(config["data_dir"], "/var/lib/chillcenter/data")
        self.assertEqual(config["video_root"], "/srv/volumes/video")
        self.assertNotIn(config["listen"].split(":", 1)[0], {"0.0.0.0", "::"})
        unit = (REPOSITORY / "deploy/linux/chillcenter.service").read_text()
        self.assertIn("User=chillcenter", unit)
        self.assertIn("ProtectSystem=strict", unit)
        self.assertIn("ReadWritePaths=/var/lib/chillcenter", unit)
        self.assertIn("ReadOnlyPaths=/srv/volumes/video", unit)
        self.assertIn("ConditionPathIsMountPoint=/srv/volumes/video", unit)
        self.assertNotIn("libmpv", unit.lower())
        self.assertNotIn("ExecStartPre=", unit)

    def test_elf_targets_are_checked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "server"
            path.write_bytes(elf(183))
            good = subprocess.run(
                [str(HELPER), "validate-elf", "--path", str(path), "--target", "aarch64-unknown-linux-gnu"],
                check=False,
            )
            bad = subprocess.run(
                [str(HELPER), "validate-elf", "--path", str(path), "--target", "armv7-unknown-linux-gnueabihf"],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(good.returncode, 0)
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn("architecture mismatch", bad.stderr)

    def test_fake_linux_host_build_produces_static_archive_without_installing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary = pathlib.Path(directory)
            tools = temporary / "tools"
            tools.mkdir()
            target = temporary / "target"
            output = temporary / "chillcenter.tar.gz"
            log = temporary / "cargo.log"
            self._tool(tools, "uname", '#!/bin/sh\n[ "$1" = "-s" ] && echo Linux || echo x86_64\n')
            self._tool(tools, "rustc", '#!/bin/sh\n[ "$1" = "-vV" ] && printf "rustc 1.99.0\\nhost: x86_64-unknown-linux-gnu\\n" || echo "rustc 1.99.0"\n')
            self._tool(tools, "rustup", '#!/bin/sh\necho x86_64-unknown-linux-gnu\n')
            self._tool(
                tools,
                "git",
                '#!/bin/sh\ncase "$*" in *"status --porcelain"*) exit 0;; *"rev-parse HEAD"*) echo 0123456789abcdef0123456789abcdef01234567;; esac\n',
            )
            cargo = f'''#!/bin/sh
if [ "$1" = "--version" ]; then echo "cargo 1.99.0"; exit 0; fi
echo "$@" >"{log}"
mkdir -p "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release"
python3 -c 'import pathlib; p=pathlib.Path("'$CARGO_TARGET_DIR'/x86_64-unknown-linux-gnu/release/chill-server"); p.write_bytes(bytes.fromhex("7f454c460201") + bytes(12) + (62).to_bytes(2,"little") + bytes(44))'
chmod 755 "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/chill-server"
'''
            self._tool(tools, "cargo", cargo)
            environment = os.environ.copy()
            environment["PATH"] = f"{tools}:{environment['PATH']}"
            environment["CARGO_TARGET_DIR"] = str(target)
            result = subprocess.run(
                [str(SCRIPT), "--target", "host-linux", "--output", str(output)],
                cwd=REPOSITORY,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            invocation = log.read_text(encoding="utf-8")
            self.assertIn("--release --locked --offline", invocation)
            self.assertIn("--target x86_64-unknown-linux-gnu", invocation)
            self.assertTrue(output.is_file())
            checksum = pathlib.Path(f"{output}.sha256")
            self.assertTrue(checksum.is_file())
            self.assertIn(output.name, checksum.read_text(encoding="utf-8"))
            with tarfile.open(output, "r:gz") as archive:
                names = archive.getnames()
                binary_name = next(name for name in names if name.endswith("/usr/lib/chillcenter/chill-server"))
                self.assertEqual(archive.getmember(binary_name).mode, 0o755)
                self.assertTrue(any(name.endswith("/usr/lib/systemd/system/chillcenter.service") for name in names))
                self.assertTrue(any(name.endswith("/etc/chillcenter/server.json.example") for name in names))
                manifest_name = next(name for name in names if name.endswith("/build-manifest.json"))
                manifest = json.load(archive.extractfile(manifest_name))
                self.assertEqual(manifest["target"], "x86_64-unknown-linux-gnu")
                self.assertEqual(manifest["source_commit"], "0123456789abcdef0123456789abcdef01234567")
                self.assertFalse(manifest["runtime"]["deployment_performed"])
                self.assertFalse(any(name.endswith("/etc/chillcenter/server.json") for name in names))
                self.assertFalse(any("secret" in name.lower() or name.endswith(".pem") for name in names))

    def test_refuses_existing_output_before_build(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory) / "existing.tar.gz"
            output.write_bytes(b"keep")
            result = subprocess.run(
                [str(SCRIPT), "--target", "host-linux", "--output", str(output)],
                cwd=REPOSITORY,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(output.read_bytes(), b"keep")

    @staticmethod
    def _tool(directory: pathlib.Path, name: str, body: str) -> None:
        path = directory / name
        path.write_text(body, encoding="utf-8")
        path.chmod(path.stat().st_mode | stat.S_IXUSR)


if __name__ == "__main__":
    unittest.main()
