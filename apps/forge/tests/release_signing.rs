//! Structure of the signing steps in the release workflow (docs/RELEASING.md, brief 177).
//! The real tools (codesign, notarytool, signtool, gpg) are never run here.

use std::path::Path;

fn read(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel),
    )
    .unwrap()
}

/// Splits the workflow into steps: a step starts at a line `      - ` (six spaces).
fn steps(wf: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in wf.lines() {
        if line.starts_with("      - ") {
            out.push(String::new());
        }
        if let Some(last) = out.last_mut() {
            last.push_str(line);
            last.push('\n');
        }
    }
    out
}

const SIGN_WORDS: &[&str] = &[
    "codesign",
    "notarytool",
    "stapler",
    "signtool",
    "gpg ",
    "security import",
    "security create-keychain",
    "FromBase64String",
];

fn is_signing(step: &str) -> bool {
    SIGN_WORDS.iter().any(|w| step.contains(w))
}

#[test]
fn every_signing_step_is_gated_on_a_secret_check_output() {
    let wf = read(".github/workflows/release.yml");
    let all = steps(&wf);
    let signing: Vec<&String> = all.iter().filter(|s| is_signing(s)).collect();
    assert!(
        signing.len() >= 5,
        "expected the signing steps, found {}",
        signing.len()
    );
    for s in signing {
        let cond = s
            .lines()
            .find(|l| l.trim_start().starts_with("if:") || l.trim_start().starts_with("- if:"))
            .unwrap_or_else(|| panic!("signing step without `if:`:\n{s}"));
        assert!(
            cond.contains("steps.") && cond.contains(".outputs.") || cond.contains("always()"),
            "signing step not gated on a gate output:\n{s}"
        );
    }
    for gate in ["mac_gate", "win_gate", "gpg_gate"] {
        let g = all
            .iter()
            .find(|s| s.contains(&format!("id: {gate}")))
            .unwrap_or_else(|| panic!("missing gate step {gate}"));
        assert!(
            g.contains("secrets.") && g.contains("!= ''"),
            "{gate} must test secrets != ''"
        );
        assert!(
            g.contains("signing skipped:"),
            "{gate} must log the skip reason"
        );
    }
}

#[test]
fn secrets_are_only_used_in_gates_and_env_never_run_or_echoed() {
    let wf = read(".github/workflows/release.yml");
    for s in steps(&wf) {
        // A secret expression may only appear as an `env:` value or in a `!= ''` gate check
        // (which interpolates a boolean, never the value).
        for line in s.lines().filter(|l| l.contains("secrets.")) {
            let t = line.trim_start();
            let env_value = t.contains(": ${{ secrets.") && t.ends_with("}}");
            let gate_check = t.contains("!= ''") && t.contains("= \"true\"");
            assert!(
                env_value || gate_check,
                "secret not passed via env: or gate: {line}"
            );
        }
        // No echo/printf of a secret variable to the log.
        for line in s.lines() {
            let t = line.trim();
            if (t.starts_with("echo") || t.starts_with("printf") || t.starts_with("Write-Host"))
                && !t.contains('|')
                && !t.contains('>')
                && !t.contains("::add-mask::")
            {
                for v in [
                    "MACOS_CERT_P12",
                    "MACOS_CERT_PASSWORD",
                    "MACOS_NOTARY_KEY_P8",
                    "WINDOWS_CERT_PFX",
                    "WINDOWS_CERT_PASSWORD",
                    "LINUX_GPG_PRIVATE_KEY",
                    "LINUX_GPG_PASSPHRASE",
                    "KEYCHAIN_PASSWORD",
                ] {
                    let expanded = t.contains(&format!("${v}"))
                        || t.contains(&format!("${{{v}}}"))
                        || t.contains(&format!("env:{v}"));
                    assert!(!expanded, "secret {v} echoed: {t}");
                }
            }
        }
    }
    for script in [
        "scripts/release/sign-hooks.sh",
        "scripts/release/sign-hooks.ps1",
    ] {
        let text = read(script);
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with("echo") || t.starts_with("Write-Host") {
                assert!(
                    !t.contains("$env:") && !t.contains("PASSWORD}"),
                    "{script}: {t}"
                );
                for v in ["PASSWORD", "PASSPHRASE", "P12", "PFX", "KEY_P8"] {
                    assert!(
                        !t.contains(&format!("${v}")) && !t.contains(&format!("env:{v}")),
                        "{script} echoes a secret ({v}): {t}"
                    );
                }
            }
        }
    }
}

#[test]
fn cleanup_steps_run_always() {
    let wf = read(".github/workflows/release.yml");
    let all = steps(&wf);
    for gate in ["mac_gate", "win_gate", "gpg_gate"] {
        let cleanups: Vec<&String> = all
            .iter()
            .filter(|s| s.contains("name: Cleanup") && s.contains(gate))
            .collect();
        assert_eq!(cleanups.len(), 1, "one cleanup step for {gate}");
        let c = cleanups[0];
        assert!(
            c.contains("if: always() &&"),
            "cleanup for {gate} must use always():\n{c}"
        );
        assert!(
            c.contains("rm -rf") || c.contains("Remove-Item"),
            "cleanup deletes files:\n{c}"
        );
    }
    let mac = all
        .iter()
        .find(|s| s.contains("name: Cleanup macOS"))
        .unwrap();
    assert!(mac.contains("security delete-keychain"));
}

#[test]
fn unsigned_path_still_uploads_artifacts() {
    let wf = read(".github/workflows/release.yml");
    let all = steps(&wf);
    // Uploads are never gated on a signing output and always error if empty.
    let uploads: Vec<&String> = all
        .iter()
        .filter(|s| s.contains("actions/upload-artifact") && !s.contains("changelog"))
        .collect();
    assert!(uploads.len() >= 4, "per-OS uploads plus the bundle");
    for u in &uploads {
        assert!(!u.contains("if:"), "upload must be unconditional:\n{u}");
    }
    for name in ["linux-x86_64", "macos-universal2", "windows-x86_64"] {
        let u = uploads.iter().find(|u| u.contains(name)).unwrap();
        assert!(u.contains("if-no-files-found: error"), "{name}");
    }
    assert!(
        wf.contains("dist/*.asc"),
        "signatures are uploaded when present"
    );
    // The signing steps never abort the job when secrets are absent: the package steps carry no
    // unconditional secret requirement and scripts skip on missing input.
    let sh = read("scripts/release/sign-hooks.sh");
    assert!(sh.contains("signing skipped:"));
    let ps = read("scripts/release/sign-hooks.ps1");
    assert!(ps.contains("signing skipped:"));
}

#[test]
fn permissions_stay_minimal_and_actions_are_pinned() {
    let wf = read(".github/workflows/release.yml");
    assert!(wf.contains("\npermissions:\n  contents: read"));
    assert_eq!(
        wf.matches("contents: write").count(),
        1,
        "only the publish job writes"
    );
    assert!(!wf.contains("id-token") && !wf.contains("permissions: write-all"));
    for line in wf.lines().filter(|l| l.contains("uses:")) {
        assert!(line.contains('@'), "unpinned action: {line}");
    }
}
