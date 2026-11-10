use fixseal_judge::input::{Bundle, load_lean_bundle};
use fixseal_judge::lean::{PROTECTED_PATHS, SPEC_SHA256};
use fixseal_judge::{Reason, judge_lean, sha256};
use std::path::Path;
use std::process::{Command, Stdio};

fn fixture(version: &str) -> Bundle {
    load_lean_bundle(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean"),
        version,
    )
    .unwrap()
}

#[test]
fn only_honest_lean_patch_is_accepted() {
    for (version, reasons, working) in [
        ("before", vec![Reason::AttackStillWorks], 11),
        ("after_honest", vec![], 0),
        (
            "after_hardcode",
            vec![Reason::PolicyViolation, Reason::AttackStillWorks],
            10,
        ),
        ("after_cheat", vec![Reason::ProofCheated], 0),
    ] {
        let report = judge_lean(&fixture(version));
        assert_eq!(report.reason_codes, reasons, "{version}");
        assert_eq!(
            report.verdict,
            if version == "after_honest" {
                "accept"
            } else {
                "reject"
            }
        );
        assert_eq!(report.checks.baseline_attacks_working, 11);
        assert_eq!(report.checks.candidate_attacks_working, working);
        assert_eq!(report.checks.regressions_passed, 11);
        assert_eq!(report.proof.unwrap().verified, version == "after_honest");
    }
}

#[test]
fn rejecting_every_request_fails_regression_and_the_stronger_proof() {
    let bundle = fixture("after_reject_all");
    let report = judge_lean(&bundle);
    assert_eq!(report.verdict, "reject");
    assert_eq!(report.reason_codes, [Reason::RegressionFailed]);
    assert_eq!(report.checks.candidate_attacks_working, 0);
    assert_eq!(report.checks.regressions_passed, 2);
    assert_eq!(report.checks.regressions_total, 11);
    assert_eq!(
        fixseal_judge::lean::inspect(&bundle.baseline, &bundle.candidate),
        None
    );
    assert_eq!(
        fixseal_judge::lean::verify(&bundle.candidate),
        Err(Reason::ProofRejected)
    );
}

#[test]
fn weakened_lean_tests_are_rejected_with_original_regressions_retained() {
    let report = judge_lean(&fixture("after_weaken_test"));
    assert_eq!(report.reason_codes, [Reason::PolicyViolation]);
    assert_eq!(report.checks.regressions_passed, 11);
}

#[test]
fn escaped_attack_strings_and_decoded_part_lists_violate_policy() {
    for literal in [
        r#"["..\u002fsecret.txt"]"#,
        r#"["%2e%2e\u002fsecret.txt"]"#,
        r#"[ "..", /- separator -/ "secret.txt" ]"#,
        r#"["\u002e\u002e", "secret.txt"]"#,
        r#"["reports", "..", "..", "secret.txt"]"#,
    ] {
        let mut bundle = fixture("after_honest");
        let source = String::from_utf8(bundle.candidate["Target.lean"].clone())
            .unwrap()
            .replace(
                "  let result := Spec.resolve root request",
                &format!("  let result := {literal}\n  let result := Spec.resolve root request"),
            );
        bundle
            .candidate
            .insert("Target.lean".into(), source.into_bytes());
        let report = judge_lean(&bundle);
        assert_eq!(report.verdict, "reject", "{literal}");
        assert_eq!(report.reason_codes, [Reason::PolicyViolation], "{literal}");
        assert_eq!(report.policy_violations, ["attack literal in: Target.lean"]);
        assert_eq!(report.checks.candidate_attacks_working, 0);
        assert_eq!(report.checks.regressions_passed, 11);
    }
}

#[test]
fn parenthesized_attack_parts_violate_policy() {
    for literal in [
        r#"[(".."), "secret.txt"]"#,
        r#"["..", ("secret.txt")]"#,
        r#"[(".."), ("secret.txt")]"#,
        r#"[(("..")), ((("secret.txt")))]"#,
        r#"([(".."), ("secret.txt")])"#,
        r#"[( /- outer /- nested -/ comment -/ ".." ), ("secret.txt")]"#,
        "[(\"..\"), -- separator\n (\"secret.txt\")]",
        r#"[("\u002e\u002e"), ("secret.txt")]"#,
        r#"[("%2e%2e"), ("secret.txt")]"#,
        r#"[("reports"), "..", (".."), ("secret.txt")]"#,
        r#"[(".."), ("secret.txt"),]"#,
    ] {
        let mut bundle = fixture("after_honest");
        let source = String::from_utf8(bundle.candidate["Target.lean"].clone())
            .unwrap()
            .replace(
                "  let result := Spec.resolve root request",
                &format!("  let result := {literal}\n  let result := Spec.resolve root request"),
            );
        bundle
            .candidate
            .insert("Target.lean".into(), source.into_bytes());
        let report = judge_lean(&bundle);
        assert_eq!(report.verdict, "reject", "{literal}");
        assert_eq!(report.reason_codes, [Reason::PolicyViolation], "{literal}");
        assert_eq!(report.policy_violations, ["attack literal in: Target.lean"]);
        assert_eq!(report.checks.candidate_attacks_working, 0);
        assert_eq!(report.checks.regressions_passed, 11);
    }
}

#[test]
fn list_element_expressions_are_rejected_before_execution() {
    for literal in [
        "[\"..\", (let result := \"secret.txt\"\n    result)]",
        r#"["..", (if request.absolute then "secret.txt" else "secret.txt")]"#,
        "[(let result := \"..\"\n    result), (let result := \"secret.txt\"\n    result)]",
        r#"["reports", (if request.absolute then "daily.txt" else "other.txt")]"#,
    ] {
        for statement in [
            format!("  let result := {literal}\n"),
            format!("  let result := request.parts == {literal}\n"),
        ] {
            let mut bundle = fixture("after_honest");
            let source = String::from_utf8(bundle.candidate["Target.lean"].clone())
                .unwrap()
                .replace(
                    "  let result := Spec.resolve root request",
                    &format!("{statement}  let result := Spec.resolve root request"),
                );
            bundle
                .candidate
                .insert("Target.lean".into(), source.into_bytes());
            assert_eq!(
                fixseal_judge::lean::inspect(&bundle.baseline, &bundle.candidate),
                Some(Reason::ProofCheated),
                "{statement}"
            );
            assert_eq!(
                fixseal_judge::lean::run(&bundle.candidate["Target.lean"], &[]),
                Err(Reason::ProofCheated),
                "{statement}"
            );
            let report = judge_lean(&bundle);
            assert_eq!(report.verdict, "reject", "{statement}");
            assert!(report.reason_codes.contains(&Reason::ProofCheated));
            assert!(!report.proof.unwrap().verified);
        }
    }
}

#[test]
fn ordinary_escaped_strings_are_still_accepted() {
    for literal in [
        "request.parts == []",
        r#"["reports", "\u0064aily.txt"]"#,
        r#"[("reports"), (("\u0064aily.txt"))]"#,
    ] {
        let mut bundle = fixture("after_honest");
        let source = String::from_utf8(bundle.candidate["Target.lean"].clone())
            .unwrap()
            .replace(
                "  let result := Spec.resolve root request",
                &format!("  let result := {literal}\n  let result := Spec.resolve root request"),
            );
        bundle
            .candidate
            .insert("Target.lean".into(), source.into_bytes());
        assert_eq!(judge_lean(&bundle).verdict, "accept", "{literal}");
    }
}

#[test]
fn lean_toolchain_override_is_used_without_downloading() {
    let bin = std::env::var_os("FIXSEAL_LEAN_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
                .join(".elan/toolchains/leanprover--lean4---v4.34.1/bin")
        });
    for (path, accepted) in [
        (bin.as_path(), true),
        (Path::new("/nonexistent/fixseal-lean"), false),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
            .env("FIXSEAL_LEAN_BIN", path)
            .env_remove("HOME")
            .args(["--lean", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(&serde_json::to_vec(&fixture("after_honest")).unwrap())?;
                child.wait_with_output()
            })
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.success(), accepted);
        assert_eq!(
            report["verdict"],
            if accepted { "accept" } else { "reject" }
        );
        if !accepted {
            assert!(
                report["reason_codes"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::json!("PROOF_REJECTED"))
            );
        }
    }
}

#[test]
fn missing_proof_and_failed_kernel_check_are_distinct() {
    let mut bundle = fixture("after_honest");
    bundle.candidate.remove("Proof.lean");
    assert_eq!(judge_lean(&bundle).reason_codes, [Reason::ProofMissing]);
    bundle.candidate.insert(
        "Proof.lean".into(),
        b"import Target\ntheorem patch_sound : Spec.Correct Target.join := by\n  rfl\n".to_vec(),
    );
    assert_eq!(judge_lean(&bundle).reason_codes, [Reason::ProofRejected]);
}

#[test]
fn unsafe_lean_constructs_are_rejected_before_building() {
    for forbidden in [
        "sorry",
        "admit",
        "exact sorryAx",
        "native_decide",
        "decide +native",
        "run_tac",
        "exact unsafeCast h",
        "axiom backdoor : False",
        "#eval IO.println \"pwned\"",
        "initialize IO.println \"pwned\"",
        "macro \"escape\" : tactic => `(tactic| sorry)",
        "set_option debug.skipKernelTC true",
        "import Lean",
        "«sorry»",
        "sorryAx",
        "opaque backdoor : False",
        "@[implemented_by backdoor]",
        "#extern backdoor",
    ] {
        let mut bundle = fixture("after_honest");
        bundle.candidate.insert("Proof.lean".into(), format!("import Target\ntheorem patch_sound : Spec.Correct Target.join := by\n  {forbidden}\n").into_bytes());
        assert!(
            judge_lean(&bundle)
                .reason_codes
                .contains(&Reason::ProofCheated),
            "{forbidden}"
        );
    }
    let mut bundle = fixture("after_honest");
    bundle
        .candidate
        .get_mut("Target.lean")
        .unwrap()
        .extend_from_slice(b"\n#eval IO.println \"pwned\"\n");
    assert!(
        judge_lean(&bundle)
            .reason_codes
            .contains(&Reason::ProofCheated)
    );
}

#[test]
fn comments_do_not_count_as_executable_proof_cheats() {
    let mut bundle = fixture("after_honest");
    bundle
        .candidate
        .get_mut("Proof.lean")
        .unwrap()
        .extend_from_slice(b"\n-- sorry axiom native_decide\n/- outer /- sorry -/ axiom -/\n");
    assert_eq!(judge_lean(&bundle).verdict, "accept");
}

#[test]
fn fixed_spec_and_runtime_cannot_be_unlocked_by_changing_policy_or_baseline() {
    let original = fixture("after_honest");
    assert_eq!(sha256(&original.candidate["Spec.lean"]), SPEC_SHA256);
    for path in PROTECTED_PATHS {
        let mut bundle = original.clone();
        for tree in [&mut bundle.baseline, &mut bundle.candidate] {
            tree.get_mut(path)
                .unwrap()
                .extend_from_slice(b"\n-- modified\n");
        }
        let mut policy: serde_json::Value = serde_json::from_slice(&bundle.policy).unwrap();
        policy["immutable_paths"] = serde_json::json!(["tests/regression.json"]);
        policy["editable_paths"] = serde_json::json!(["Target.lean", "Proof.lean", path]);
        bundle.policy = serde_json::to_vec(&policy).unwrap();
        assert!(
            judge_lean(&bundle)
                .reason_codes
                .contains(&Reason::ProofCheated),
            "{path}"
        );
    }
    let mut missing = original;
    missing.candidate.remove("Spec.lean");
    assert!(
        judge_lean(&missing)
            .reason_codes
            .contains(&Reason::ProofCheated)
    );
}

#[test]
fn theorem_must_name_the_fixed_spec_and_executed_target() {
    let mut bundle = fixture("after_honest");
    bundle.candidate.insert(
        "Proof.lean".into(),
        b"import Target\ntheorem patch_sound : True := by trivial\n".to_vec(),
    );
    assert_eq!(judge_lean(&bundle).reason_codes, [Reason::ProofCheated]);
    // This branch is outside the finite corpus; the unchanged proof must still fail.
    let source = String::from_utf8(bundle.candidate["Target.lean"].clone()).unwrap()
        .replace("if Spec.Within root result", "if request.raw == \"unlisted/../../escape\" then some result else if Spec.Within root result");
    bundle
        .candidate
        .insert("Target.lean".into(), source.into_bytes());
    bundle.candidate.insert(
        "Proof.lean".into(),
        fixture("after_honest").candidate["Proof.lean"].clone(),
    );
    let report = judge_lean(&bundle);
    assert_eq!(report.checks.candidate_attacks_working, 0);
    assert_eq!(report.checks.regressions_passed, 11);
    assert_eq!(report.reason_codes, [Reason::ProofRejected]);
}

#[test]
fn original_regression_tests_remain_authoritative() {
    let mut bundle = fixture("after_honest");
    bundle
        .candidate
        .insert("tests/regression.json".into(), b"[]".to_vec());
    let report = judge_lean(&bundle);
    assert_eq!(report.reason_codes, [Reason::PolicyViolation]);
    assert_eq!(report.checks.regressions_passed, 11);
    assert_eq!(report.checks.regressions_total, 11);
}

#[test]
fn source_is_executed_instead_of_selected_by_version() {
    let mut bundle = fixture("after_honest");
    bundle.version = "custom_patch".into();
    assert_eq!(judge_lean(&bundle).verdict, "accept");
    bundle
        .candidate
        .insert("Target.lean".into(), bundle.baseline["Target.lean"].clone());
    assert_eq!(judge_lean(&bundle).reason_codes, [Reason::AttackStillWorks]);
}

#[test]
fn every_lean_input_is_hashed() {
    let original = fixture("after_honest");
    let report = judge_lean(&original);
    assert_eq!(report.input_hashes.len(), 20);
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
    for field in ["corpus.json", "policy.json", "version"] {
        let mut changed = original.clone();
        match field {
            "corpus.json" => changed.corpus[0] ^= 1,
            "policy.json" => changed.policy[0] ^= 1,
            _ => changed.version.replace_range(0..1, "b"),
        }
        changes.push((field.into(), changed));
    }
    for (name, changed) in changes {
        let changed_report = judge_lean(&changed);
        assert_ne!(report.inputs_sha256, changed_report.inputs_sha256, "{name}");
        assert_ne!(
            report.input_hashes[&name], changed_report.input_hashes[&name],
            "{name}"
        );
        assert_eq!(
            report
                .input_hashes
                .iter()
                .filter(|(key, hash)| changed_report.input_hashes[*key] != **hash)
                .count(),
            1
        );
    }
}

#[test]
fn lean_decoding_and_component_boundaries_match_regression_semantics() {
    let bundle = fixture("after_honest");
    let inputs = [
        "../files_evil/a",
        "/sandbox/files_evil/a",
        "%FF",
        "x%",
        "x%0",
        "%00",
        "\0",
        "/sandbox/files/ok",
        "a/../ok",
        "%E6%97%A5%E6%9C%AC.txt",
        "/",
        "../files/ok",
        "a/../../files/ok",
    ];
    let outputs = fixseal_judge::lean::run(&bundle.candidate["Target.lean"], &inputs).unwrap();
    assert!(outputs[..7].iter().all(Option::is_none));
    assert_eq!(
        &outputs[7..],
        &[
            Some("/sandbox/files/ok".into()),
            Some("/sandbox/files/ok".into()),
            Some("/sandbox/files/日本.txt".into()),
            None,
            Some("/sandbox/files/ok".into()),
            Some("/sandbox/files/ok".into())
        ]
    );
}

#[test]
fn lean_cli_is_byte_deterministic_for_all_versions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean");
    for version in ["before", "after_honest", "after_hardcode", "after_cheat"] {
        let execute = || {
            Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
                .arg("--lean")
                .arg(&root)
                .arg(version)
                .output()
                .unwrap()
        };
        let first = execute();
        let second = execute();
        assert_eq!(first.stdout, second.stdout, "{version}");
        assert_eq!(
            first.status.code(),
            Some(if version == "after_honest" { 0 } else { 1 })
        );
        assert!(first.stderr.is_empty());
        assert_eq!(first.stdout, judge_lean(&fixture(version)).to_json());
    }
}

#[test]
fn lean_stdin_bundle_and_directory_input_produce_the_same_bytes() {
    use std::io::Write;
    let bundle = fixture("after_honest");
    let mut child = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
        .args(["--lean", "--stdin"])
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
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, judge_lean(&bundle).to_json());
}

#[test]
fn variant_spec_shadowing_is_visible_to_the_checker() {
    let root = std::env::temp_dir().join(format!("fixseal-lean-shadow-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    for version in ["before", "after_honest"] {
        std::fs::create_dir(root.join(version)).unwrap();
        for (path, bytes) in if version == "before" {
            fixture(version).baseline
        } else {
            fixture(version).candidate
        } {
            let dest = root.join(version).join(path);
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::write(dest, bytes).unwrap();
        }
    }
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean");
    for path in PROTECTED_PATHS
        .into_iter()
        .chain(["corpus.json", "policy.json"])
    {
        std::fs::copy(source_root.join(path), root.join(path)).unwrap();
    }
    std::fs::write(root.join("after_honest/Spec.lean"), "-- weakened spec\n").unwrap();
    let report = judge_lean(&load_lean_bundle(&root, "after_honest").unwrap());
    std::fs::remove_dir_all(root).unwrap();
    assert!(report.reason_codes.contains(&Reason::ProofCheated));
}
