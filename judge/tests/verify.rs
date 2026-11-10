use fixseal_judge::bundle::{self, ChainRecord, Mode};
use fixseal_judge::input::{load_bundle, load_lean_bundle};
use serde_json::json;
use std::path::Path;
use std::process::Command;

fn evidence(mode: Mode, version: &str) -> bundle::Evidence {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let input = match mode {
        Mode::Vm => load_bundle(&root.join("target"), version),
        Mode::Lean => load_lean_bundle(&root.join("lean"), version),
    }
    .unwrap();
    bundle::create(input, mode)
}

#[test]
fn accepts_replayed_accept_and_reject_in_both_modes() {
    for mode in [Mode::Vm, Mode::Lean] {
        let versions: &[&str] = match mode {
            Mode::Vm => &["after_honest", "after_hardcode", "after_weaken_test"],
            Mode::Lean => &[
                "after_honest",
                "after_hardcode",
                "after_weaken_test",
                "after_cheat",
                "after_reject_all",
            ],
        };
        for version in versions {
            let bundle = evidence(mode, version);
            assert!(bundle::verify(&bundle).0, "{version}");
        }
    }
}

#[test]
fn detects_input_report_result_and_chain_tampering() {
    let original = evidence(Mode::Vm, "after_honest");
    for change in 0..10 {
        let mut changed: bundle::Evidence = serde_json::from_value(json!(original)).unwrap();
        changed.chain = Some(ChainRecord {
            bundle_hash: changed.report_sha256.clone(),
            caller: "agent.near".into(),
            block_timestamp_ms: "123".into(),
            summary: bundle::summary(&changed.report),
        });
        assert!(bundle::verify(&changed).0);
        match change {
            0 => changed.input.candidate.get_mut("path.fn").unwrap()[0] ^= 1,
            1 => changed.report["verdict"] = json!("reject"),
            2 => changed.report["checks"]["regressions_passed"] = json!(0),
            3 => changed.report["input_hashes"]["version"] = json!("00"),
            4 => changed.report["inputs_sha256"] = json!("00"),
            5 => changed.report_sha256 = "00".into(),
            6 => changed.chain.as_mut().unwrap().bundle_hash = "00".into(),
            7 => changed.chain.as_mut().unwrap().summary = "accept".into(),
            8 => changed.format = "unknown".into(),
            _ => changed.mode = Mode::Lean,
        }
        let (valid, lines) = bundle::verify(&changed);
        assert!(!valid, "change={change}: {lines:?}");
        assert!(lines.iter().any(|line| line.starts_with("MISMATCH")));
    }
}

#[test]
fn rehashed_lean_proof_cheat_cannot_retain_an_accept_report() {
    let mut changed = evidence(Mode::Lean, "after_honest");
    changed.input.candidate.insert(
        "Proof.lean".into(),
        b"import Target\ntheorem patch_sound : Spec.Correct Target.join := by\n  sorry\n".to_vec(),
    );
    let (hashes, hash) = fixseal_judge::hash_inputs(&changed.input);
    changed.report["input_hashes"] = json!(hashes);
    changed.report["inputs_sha256"] = json!(hash);
    let (valid, lines) = bundle::verify(&changed);
    assert!(!valid);
    assert!(lines.iter().any(|line| line.contains("PROOF_CHEATED")));
}

#[test]
fn cli_verifies_exported_bundle_and_fails_on_tampering() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target");
    let output = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
        .args(["bundle", root.to_str().unwrap(), "after_honest"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let path = std::env::temp_dir().join(format!("fixseal-verify-{}.json", std::process::id()));
    std::fs::write(&path, &output.stdout).unwrap();
    let verify = || {
        Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
            .arg("verify")
            .arg(&path)
            .output()
            .unwrap()
    };
    let checked = verify();
    assert!(checked.status.success());
    assert!(String::from_utf8_lossy(&checked.stdout).contains("VERIFIED"));
    let mut value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    value["report"]["verdict"] = json!("reject");
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let checked = verify();
    assert_eq!(checked.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&checked.stdout).contains("MISMATCH replayed verdict"));
    std::fs::write(&path, b"{}").unwrap();
    assert_eq!(verify().status.code(), Some(2));
    std::fs::remove_file(path).unwrap();
}
