//! Opt-in native SDK journey using existing published fixtures. No network,
//! production app replacement, family settings or release private key.
#![cfg(target_os = "macos")]
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};
use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

fn wait_for_successful_exit(pid: u32) -> i32 {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    // Only the isolated replacement PID just observed by this test. Darwin
    // permits NOTE_EXITSTATUS for children or same-UID signal targets.
    let raw = unsafe { libc::kqueue() };
    assert!(raw >= 0);
    let queue = unsafe { OwnedFd::from_raw_fd(raw) };
    let change = libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT | libc::NOTE_EXITSTATUS,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    let timeout = libc::timespec {
        tv_sec: 15,
        tv_nsec: 0,
    };
    let mut event = std::mem::MaybeUninit::<libc::kevent>::uninit();
    let count = unsafe {
        libc::kevent(
            queue.as_raw_fd(),
            &change,
            1,
            event.as_mut_ptr(),
            1,
            &timeout,
        )
    };
    assert_eq!(
        count,
        1,
        "cannot observe test PID exit: {}",
        std::io::Error::last_os_error()
    );
    let event = unsafe { event.assume_init() };
    let flags = event.flags;
    let notes = event.fflags;
    let status = event.data as i32;
    assert_eq!(
        flags & libc::EV_ERROR,
        0,
        "kernel rejected PID observation: {status}"
    );
    assert_ne!(
        notes & libc::NOTE_EXITSTATUS,
        0,
        "kernel did not provide exit status"
    );
    assert!(
        libc::WIFEXITED(status),
        "replacement terminated by signal: {status}"
    );
    assert_eq!(
        libc::WEXITSTATUS(status),
        0,
        "replacement executable failed"
    );
    status
}

fn tree_digest(root: &Path) -> Vec<u8> {
    fn walk(root: &Path, path: &Path, hash: &mut Sha256) {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(path).unwrap();
        hash.update(
            path.strip_prefix(root)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes(),
        );
        hash.update(metadata.mode().to_le_bytes());
        if metadata.file_type().is_symlink() {
            hash.update(fs::read_link(path).unwrap().as_os_str().as_encoded_bytes());
        } else if metadata.is_dir() {
            let mut children = fs::read_dir(path)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                walk(root, &child, hash);
            }
        } else {
            hash.update(fs::read(path).unwrap());
        }
    }
    let mut hash = Sha256::new();
    walk(root, root, &mut hash);
    hash.finalize().to_vec()
}

#[test]
#[ignore = "requires CHILL_MAC_UPDATE_QA_ROOT with preserved published 0.7.4/0.7.5 fixtures"]
fn published_package_rejects_tampering_replaces_isolated_app_and_launches() {
    let root = std::path::PathBuf::from(
        std::env::var_os("CHILL_MAC_UPDATE_QA_ROOT").expect("isolated QA parent required"),
    );
    let old = root.join("release-0.7.4/Chill Center.app");
    let release = root.join("release-0.7.5-final");
    assert!(old.join("Contents/MacOS/chillcenter-desktop").is_file());
    let work = tempfile::Builder::new()
        .prefix("mac-update-sdk.")
        .tempdir_in(&root)
        .unwrap()
        .keep();
    let fake = work.join("Chill Center.app");
    assert!(
        Command::new("/usr/bin/ditto")
            .arg(&old)
            .arg(&fake)
            .status()
            .unwrap()
            .success()
    );
    let before = tree_digest(&fake);
    let bytes = fs::read(release.join("Chill-Center-0.7.5-macos-arm64.app.tar.gz")).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(release.join("mac/mac-manifest.json")).unwrap()).unwrap();
    let signature = manifest["assets"][0]["tauri_signature"].as_str().unwrap();
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.package_info_mut().version = semver::Version::new(0, 7, 4);
    let configuration: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    context.config_mut().plugins.0.insert(
        "updater".into(),
        configuration["plugins"]["updater"].clone(),
    );
    let app = tauri::test::mock_builder()
        .plugin(
            tauri_plugin_updater::Builder::new()
                .pubkey(STANDARD.encode(include_str!("../../../updates/release.pub")))
                .build(),
        )
        .build(context)
        .unwrap();
    let updater = app
        .handle()
        .updater_builder()
        .target("darwin-aarch64")
        .executable_path(fake.join("Contents/MacOS/chillcenter-desktop"))
        .endpoints(vec!["https://example.invalid/unused".parse().unwrap()])
        .unwrap()
        .build()
        .unwrap();
    let update = updater.restore_update(serde_json::json!({"version":"0.7.5","platforms":{"darwin-aarch64":{"url":"https://example.invalid/unused-package","signature":signature}}})).unwrap().unwrap();
    let mut corrupted = bytes.clone();
    corrupted[100] ^= 1;
    assert!(update.install(&corrupted).is_err());
    assert_eq!(
        tree_digest(&fake),
        before,
        "tampered update must preserve every app byte and mode"
    );
    update.verify(&bytes).unwrap();
    update.install(&bytes).unwrap();
    assert_ne!(tree_digest(&fake), before);
    assert!(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&fake)
            .status()
            .unwrap()
            .success()
    );
    // This entrypoint exits before opening Tauri or the user's settings. It
    // actually executes the replaced Mach-O and loads the bundled libmpv.
    let result = Command::new(fake.join("Contents/MacOS/chillcenter-desktop"))
        .arg("--verify-bundle")
        .env("HOME", &work)
        .env("TMPDIR", &work)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("BUNDLE_LIBRARY_OK"));
    fs::write(work.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","tampered_bytes_rejected":true,"failed_install_preserved_tree":true,"real_sdk_replacement":true,"replaced_executable_launched":true,"codesign":true,"production_settings_accessed":false,"app_restart_api_exercised":false})).unwrap()).unwrap();
    println!("SDK journey PASS: {}", work.display());
}

#[test]
#[ignore = "child of real_restart_api; never run against a production bundle"]
fn restart_child() {
    let Some(directory) = std::env::var_os("CHILL_MAC_RESTART_CHILD") else {
        return;
    };
    let work = std::path::PathBuf::from(directory).canonicalize().unwrap();
    assert!(
        work.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("mac-update-restart.")
    );
    let executable = std::env::current_exe().unwrap();
    assert_eq!(
        executable,
        work.join("Chill Center.app/Contents/MacOS/chillcenter-desktop")
    );
    let release = work.parent().unwrap().join("release-0.7.5-final");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(release.join("mac/mac-manifest.json")).unwrap()).unwrap();
    let bytes = fs::read(release.join("Chill-Center-0.7.5-macos-arm64.app.tar.gz")).unwrap();
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.package_info_mut().version = semver::Version::new(0, 7, 4);
    let configuration: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    context.config_mut().plugins.0.insert(
        "updater".into(),
        configuration["plugins"]["updater"].clone(),
    );
    // Pre-manage Env before build, without unmanaging state or unsafe mutation.
    // The replacement exits at this exact two-argument entrypoint, before any
    // settings/catalogue/UI initialization, even when launched by LaunchServices.
    let mut restart_env = tauri::Env::default();
    restart_env.args_os = vec![
        executable.clone().into_os_string(),
        "--verify-bundle".into(),
    ];
    let app = tauri::test::mock_builder()
        .manage(restart_env)
        .plugin(
            tauri_plugin_updater::Builder::new()
                .pubkey(STANDARD.encode(include_str!("../../../updates/release.pub")))
                .build(),
        )
        .build(context)
        .unwrap();
    assert_eq!(app.env().args_os.len(), 2);
    let updater = app
        .handle()
        .updater_builder()
        .target("darwin-aarch64")
        .executable_path(&executable)
        .endpoints(vec!["https://example.invalid/unused".parse().unwrap()])
        .unwrap()
        .build()
        .unwrap();
    let update = updater.restore_update(serde_json::json!({"version":"0.7.5","platforms":{"darwin-aarch64":{"url":"https://example.invalid/unused-package","signature":manifest["assets"][0]["tauri_signature"]}}})).unwrap().unwrap();
    update.install(&bytes).unwrap();
    fs::write(work.join("before-restart"), std::process::id().to_string()).unwrap();
    // This is the actual production restart API, now running from inside the
    // isolated bundle rather than the parent harness executable in target/.
    app.handle().restart();
}

#[test]
#[ignore = "requires preserved published fixtures and native LaunchServices"]
fn real_restart_api_executes_replaced_bundle() {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let root = std::path::PathBuf::from(
        std::env::var_os("CHILL_MAC_UPDATE_QA_ROOT").expect("isolated QA parent required"),
    );
    let work = tempfile::Builder::new()
        .prefix("mac-update-restart.")
        .tempdir_in(&root)
        .unwrap()
        .keep();
    let fake = work.join("Chill Center.app");
    assert!(
        Command::new("/usr/bin/ditto")
            .arg(root.join("release-0.7.4/Chill Center.app"))
            .arg(&fake)
            .status()
            .unwrap()
            .success()
    );
    let executable = fake.join("Contents/MacOS/chillcenter-desktop");
    fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
    assert!(
        Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(&fake)
            .status()
            .unwrap()
            .success()
    );
    let log = fs::File::create(work.join("child.log")).unwrap();
    let mut child = Command::new(&executable)
        .args(["--ignored", "--exact", "restart_child", "--nocapture"])
        .env("CHILL_MAC_RESTART_CHILD", &work)
        .env("HOME", &work)
        .env("TMPDIR", &work)
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .unwrap();
    let start = Instant::now();
    let restarted_pid = loop {
        if work.join("before-restart").exists() {
            let processes = Command::new("/usr/bin/pgrep")
                .arg("-f")
                .arg(&executable)
                .output()
                .unwrap();
            let candidate = String::from_utf8_lossy(&processes.stdout)
                .lines()
                .filter_map(|line| line.parse::<u32>().ok())
                .find(|pid| *pid != child.id());
            if let Some(pid) = candidate {
                break pid;
            }
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill(); // Only the test process created just above.
            panic!(
                "restart not observed in {}: {}",
                work.display(),
                fs::read_to_string(work.join("child.log")).unwrap()
            );
        }
        thread::sleep(Duration::from_millis(10));
    };
    let exit_status = wait_for_successful_exit(restarted_pid);
    assert!(child.wait().unwrap().success());
    assert!(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&fake)
            .status()
            .unwrap()
            .success()
    );
    fs::write(work.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","real_app_restart_api":true,"old_pid":child.id(),"restarted_pid":restarted_pid,"restart_argument":"--verify-bundle","production_settings_accessed":false,"restarted_exit_status":exit_status})).unwrap()).unwrap();
    println!("Native app.restart PASS: {}", work.display());
}
