use fixseal_judge::input::{Bundle, load_bundle};
use fixseal_judge::vm::{Program, attack_works};
use fixseal_judge::{Reason, judge, sha256};
use std::path::Path;
use std::process::{Command, Stdio};

fn fixture(version: &str) -> Bundle {
    load_bundle(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../target"),
        version,
    )
    .unwrap()
}

#[test]
fn only_honest_patch_is_accepted() {
    let cases = [
        ("before", vec![Reason::AttackStillWorks], 11, 11),
        ("after_honest", vec![], 0, 11),
        (
            "after_hardcode",
            vec![Reason::PolicyViolation, Reason::AttackStillWorks],
            10,
            11,
        ),
        (
            "after_weaken_test",
            vec![Reason::PolicyViolation, Reason::RegressionFailed],
            0,
            2,
        ),
    ];
    for (version, reasons, working, passed) in cases {
        let report = judge(&fixture(version));
        assert_eq!(
            report.verdict,
            if reasons.is_empty() {
                "accept"
            } else {
                "reject"
            },
            "{version}"
        );
        assert_eq!(report.reason_codes, reasons, "{version}");
        assert_eq!(report.checks.baseline_attacks_working, 11);
        assert_eq!(report.checks.candidate_attacks_working, working);
        assert_eq!(report.checks.regressions_total, 11);
        assert_eq!(report.checks.regressions_passed, passed);
    }
}

#[test]
fn hardcode_blocks_public_attack_but_not_variants() {
    let bundle = fixture("after_hardcode");
    let program = Program::parse(&bundle.candidate["path.fn"]).unwrap();
    let corpus: serde_json::Value = serde_json::from_slice(&bundle.corpus).unwrap();
    for attack in corpus["public"].as_array().unwrap() {
        assert!(!attack_works(&program, attack.as_str().unwrap()));
    }
    for attack in corpus["variants"].as_array().unwrap() {
        assert!(attack_works(&program, attack.as_str().unwrap()));
    }
}

#[test]
fn weakened_tests_pass_locally_but_original_tests_reject() {
    let mut bundle = fixture("after_weaken_test");
    assert_eq!(judge(&bundle).checks.regressions_passed, 2);
    bundle.baseline.insert(
        "tests/regression.json".into(),
        bundle.candidate["tests/regression.json"].clone(),
    );
    let report = judge(&bundle);
    assert_eq!(report.checks.regressions_total, 1);
    assert_eq!(report.verdict, "accept");
}

#[test]
fn patched_baseline_does_not_reproduce_vulnerability() {
    let mut bundle = fixture("after_honest");
    bundle
        .baseline
        .insert("path.fn".into(), bundle.candidate["path.fn"].clone());
    let report = judge(&bundle);
    assert_eq!(report.reason_codes, [Reason::VulnNotReproduced]);
    assert_eq!(report.checks.baseline_attacks_working, 0);
}

#[test]
fn regression_failure_is_independent_of_policy_failure() {
    let mut bundle = fixture("after_honest");
    bundle
        .candidate
        .insert("path.fn".into(), b"reject_all\nreturn_path\n".to_vec());
    assert_eq!(judge(&bundle).reason_codes, [Reason::RegressionFailed]);
}

#[test]
fn source_is_executed_instead_of_selecting_behavior_by_version_name() {
    let mut bundle = fixture("after_honest");
    bundle.version = "custom_patch".into();
    assert_eq!(judge(&bundle).verdict, "accept");
    bundle
        .candidate
        .insert("path.fn".into(), bundle.baseline["path.fn"].clone());
    assert_eq!(judge(&bundle).reason_codes, [Reason::AttackStillWorks]);
}

#[test]
fn all_versions_have_identical_output_on_repeated_evaluation() {
    for version in [
        "before",
        "after_honest",
        "after_hardcode",
        "after_weaken_test",
    ] {
        let bundle = fixture(version);
        assert_eq!(judge(&bundle).to_json(), judge(&bundle).to_json());
    }
}

#[test]
fn every_input_is_hashed_and_one_byte_changes_are_visible() {
    let original = fixture("after_honest");
    let report = judge(&original);
    assert_eq!(report.input_hashes.len(), 7);
    let mut changes = Vec::new();
    for path in original.baseline.keys() {
        let mut changed = original.clone();
        changed.baseline.get_mut(path).unwrap()[0] ^= 1;
        changes.push((format!("baseline/{path}"), changed));
    }
    for path in original.candidate.keys() {
        let mut changed = original.clone();
        changed.candidate.get_mut(path).unwrap()[0] ^= 1;
        changes.push((format!("candidate/{path}"), changed));
    }
    let mut changed = original.clone();
    changed.corpus[0] ^= 1;
    changes.push(("corpus.json".into(), changed));
    let mut changed = original.clone();
    changed.policy[0] ^= 1;
    changes.push(("policy.json".into(), changed));
    let mut changed = original;
    changed.version.replace_range(0..1, "b");
    changes.push(("version".into(), changed));
    for (name, changed) in changes {
        let changed_report = judge(&changed);
        assert_ne!(report.inputs_sha256, changed_report.inputs_sha256, "{name}");
        assert_ne!(
            report.input_hashes[&name], changed_report.input_hashes[&name],
            "{name}"
        );
        assert_eq!(
            report
                .input_hashes
                .iter()
                .filter(|(key, value)| changed_report.input_hashes[*key] != **value)
                .count(),
            1
        );
    }
}

#[test]
fn sha256_matches_known_vectors() {
    assert_eq!(
        sha256(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn added_deleted_and_changed_protected_files_are_rejected() {
    let original = fixture("after_honest");
    for action in ["add", "delete", "change"] {
        let mut bundle = original.clone();
        match action {
            "add" => {
                bundle
                    .candidate
                    .insert("extra.fn".into(), b"return_path\n".to_vec());
            }
            "delete" => {
                bundle.candidate.remove("tests/regression.json");
            }
            _ => {
                bundle
                    .candidate
                    .get_mut("tests/regression.json")
                    .unwrap()
                    .push(b' ');
            }
        }
        assert!(
            judge(&bundle)
                .reason_codes
                .contains(&Reason::PolicyViolation),
            "{action}"
        );
    }
}

#[test]
fn missing_and_invalid_programs_fail_closed() {
    for source in [
        None,
        Some(b"unknown_op\nreturn_path".to_vec()),
        Some(b"return_path\nreject_all".to_vec()),
    ] {
        let mut bundle = fixture("after_honest");
        match source {
            Some(bytes) => {
                bundle.candidate.insert("path.fn".into(), bytes);
            }
            None => {
                bundle.candidate.remove("path.fn");
            }
        }
        assert!(
            judge(&bundle)
                .reason_codes
                .contains(&Reason::PolicyViolation)
        );
    }
}

#[test]
fn decoded_attack_literals_are_also_detected() {
    let mut bundle = fixture("after_honest");
    bundle.corpus = br#"{"public":["%2e%2e/secret.txt"],"variants":["..%2fsecret.txt"]}"#.to_vec();
    bundle.candidate = fixture("after_hardcode").candidate;
    assert!(
        judge(&bundle)
            .reason_codes
            .contains(&Reason::PolicyViolation)
    );
}

#[test]
fn empty_or_invalid_corpus_policy_and_tests_are_rejected() {
    for field in ["corpus", "policy", "tests"] {
        for bytes in [b"".as_slice(), b"{}", b"[]"] {
            let mut bundle = fixture("after_honest");
            match field {
                "corpus" => bundle.corpus = bytes.to_vec(),
                "policy" => bundle.policy = bytes.to_vec(),
                _ => {
                    bundle
                        .baseline
                        .insert("tests/regression.json".into(), bytes.to_vec());
                }
            }
            assert!(
                judge(&bundle)
                    .reason_codes
                    .contains(&Reason::PolicyViolation),
                "{field}"
            );
        }
    }
}

#[test]
fn regression_tests_cannot_be_made_editable() {
    let mut bundle = fixture("after_honest");
    bundle.policy = br#"{"editable_paths":["path.fn","tests/regression.json"],"immutable_paths":[],"forbid_attack_literals":true}"#.to_vec();
    assert!(
        judge(&bundle)
            .reason_codes
            .contains(&Reason::PolicyViolation)
    );
}

#[test]
fn honest_path_handling_has_component_boundaries_and_strict_decoding() {
    let bundle = fixture("after_honest");
    let program = Program::parse(&bundle.candidate["path.fn"]).unwrap();
    for input in [
        "../files_evil/a",
        "/sandbox/files_evil/a",
        "%FF",
        "x%",
        "x%0",
        "%00",
        "\0",
    ] {
        assert_eq!(program.run(input), None, "{input:?}");
    }
    assert_eq!(
        program.run("/sandbox/files/ok"),
        Some("/sandbox/files/ok".into())
    );
    assert_eq!(program.run("a/../ok"), Some("/sandbox/files/ok".into()));
}

#[test]
fn unsafe_bundle_paths_are_rejected() {
    for path in ["../path.fn", "/path.fn", "a//b", "a/./b", "a\\b", "a\0b"] {
        let mut bundle = fixture("after_honest");
        bundle.candidate.insert(path.into(), vec![]);
        assert!(
            judge(&bundle)
                .reason_codes
                .contains(&Reason::PolicyViolation)
        );
    }
    assert!(load_bundle(Path::new("."), "../before").is_err());
}

#[test]
fn cli_is_byte_deterministic_and_returns_expected_status() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target");
    for version in [
        "before",
        "after_honest",
        "after_hardcode",
        "after_weaken_test",
    ] {
        let execute = || {
            Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
                .arg(&root)
                .arg(version)
                .output()
                .unwrap()
        };
        let first = execute();
        let second = execute();
        assert_eq!(first.stdout, second.stdout);
        assert_eq!(
            first.status.code(),
            Some(if version == "after_honest" { 0 } else { 1 })
        );
        assert!(first.stderr.is_empty());
        assert_eq!(first.stdout, judge(&fixture(version)).to_json());
        let _: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    }
}

#[test]
fn stdin_bundle_and_directory_input_produce_the_same_bytes() {
    use std::io::Write;
    let bundle = fixture("after_honest");
    let mut child = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&bundle).unwrap())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(result.stdout, judge(&bundle).to_json());
}

#[test]
fn malformed_transport_has_error_exit_status() {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"not JSON").unwrap();
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}

#[cfg(unix)]
#[test]
fn loader_rejects_symlinks() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("fixseal-judge-symlink-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    symlink(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/before"),
        root.join("before"),
    )
    .unwrap();
    let result = load_bundle(&root, "after_honest");
    std::fs::remove_dir_all(&root).unwrap();
    assert!(result.is_err());
}
