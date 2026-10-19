use std::path::PathBuf;
use std::process::Command;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn installer_shell_syntax_and_server_help_are_benign() {
    let root = repository_root();
    for script in [
        "scripts/install-server.sh",
        "scripts/install-web.sh",
        "scripts/install-themes.sh",
    ] {
        let status = Command::new("bash")
            .arg("-n")
            .arg(root.join(script))
            .status()
            .expect("bash must be available for installer syntax validation");
        assert!(status.success(), "invalid shell syntax in {script}");
    }

    let output = Command::new("bash")
        .arg(root.join("scripts/install-server.sh"))
        .arg("--help")
        .output()
        .expect("server installer help must run");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("First install only"));

    let server_installer = std::fs::read_to_string(root.join("scripts/install-server.sh")).unwrap();
    for required in [
        "O_NOFOLLOW",
        "chillcenter-bootstrap.XXXXXX",
        "unit-chillcenter-service",
        "unit-chillcenter-update-service",
        "unit-chillcenter-update-timer",
        "cmp -s \"$server\"",
        "\"$MINISIGN\" -Vm \"$manifest\"",
        "readonly MINISIGN=/usr/bin/minisign",
        "\"$PYTHON\" -I -",
    ] {
        assert!(
            server_installer.contains(required),
            "bootstrap security regression: missing {required}"
        );
    }
}
