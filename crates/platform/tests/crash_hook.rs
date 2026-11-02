//! A forced panic in a child process writes a report and still reaches the previous hook
//! (docs/specs/PLATFORM.md §8.4). The test binary runs itself: the `child_*` tests do nothing
//! unless `FORGE_TEST_CRASH_CHILD` is set.

use std::path::{Path, PathBuf};
use std::process::Command;

use forge_platform::crash::{HookConfig, install_hook};

fn run_child(test: &str, dir: &Path) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env("FORGE_TEST_CRASH_CHILD", "1")
        .env("FORGE_CRASH_DIR", dir)
        .env("RUST_BACKTRACE", "0")
        .output()
        .unwrap()
}

fn child_dir() -> Option<PathBuf> {
    std::env::var_os("FORGE_TEST_CRASH_CHILD")?;
    std::env::var_os("FORGE_CRASH_DIR").map(PathBuf::from)
}

fn hook(dir: &Path) {
    install_hook(HookConfig {
        app: "crash-hook-test".into(),
        version: "0.0.1".into(),
        dir: Some(dir.to_path_buf()),
        commands: || vec!["1 object.create {\"type\":\"forge.cube\"}".into()],
        log_lines: || vec!["INFO child: about to panic".into()],
        secrets: vec!["s3cret-token".into()],
    });
}

#[test]
fn child_panics() {
    let Some(dir) = child_dir() else { return };
    forge_platform::crash::set_gpu_info("Child GPU");
    hook(&dir);
    panic!("forced panic with s3cret-token inside");
}

#[test]
fn child_panics_twice() {
    let Some(dir) = child_dir() else { return };
    hook(&dir);
    let _ = std::panic::catch_unwind(|| panic!("first"));
    let _ = std::panic::catch_unwind(|| panic!("second"));
}

#[test]
fn forced_panic_writes_a_report_and_chains_the_previous_hook() {
    let t = tempfile::tempdir().unwrap();
    let out = run_child("child_panics", t.path());
    assert!(!out.status.success(), "the child must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("forced panic"),
        "previous hook did not run: {stderr}"
    );

    let reports = forge_platform::crash::unseen(t.path());
    assert_eq!(reports.len(), 1, "{stderr}");
    let text = std::fs::read_to_string(&reports[0].path).unwrap();
    for needle in [
        "Forge3D crash report",
        "Application: crash-hook-test 0.0.1",
        "GPU: Child GPU",
        "Panic: forced panic with <redacted> inside",
        "crash_hook.rs",
        "Backtrace:",
        "object.create",
        "INFO child: about to panic",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
    assert!(!text.contains("s3cret-token"));
}

#[test]
fn every_panic_of_a_process_gets_its_own_report() {
    let t = tempfile::tempdir().unwrap();
    let out = run_child("child_panics_twice", t.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(forge_platform::crash::unseen(t.path()).len(), 2);
}
