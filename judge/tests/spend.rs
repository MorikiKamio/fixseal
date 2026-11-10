use fixseal_judge::bundle::{self, Mode};
use fixseal_judge::input::{Bundle, load_lean_subject_bundle};
use fixseal_judge::lean::{self, Subject};
use fixseal_judge::{Reason, hash_inputs, judge_lean, sha256};
use std::path::Path;
use std::process::Command;

fn fixture(version: &str) -> Bundle {
    load_lean_subject_bundle(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean"),
        version,
        Subject::Spend,
    )
    .unwrap()
}

fn with_target(expression: &str) -> Bundle {
    let mut bundle = fixture("after_honest");
    bundle.candidate.insert(
        "Target.lean".into(),
        format!(
            "import SpendSpec\nnamespace Target\ndef authorize (limit spent amount : UInt64) : Bool :=\n  {expression}\nend Target\n"
        )
        .into_bytes(),
    );
    bundle
}

#[test]
fn specification_expression_in_target_violates_policy() {
    let mut input = with_target("decide (spent.toNat + amount.toNat ≤ limit.toNat)");
    input.candidate.insert(
        "Proof.lean".into(),
        b"import Target\ntheorem patch_sound : SpendSpec.Correct Target.authorize := by\n  simp [Target.authorize]\n".to_vec(),
    );
    let report = judge_lean(&input);
    assert_eq!(report.judge_version, "0.1.0/lean-spend-4");
    assert_eq!(report.verdict, "reject");
    assert_eq!(report.reason_codes, [Reason::PolicyViolation]);
    assert!(!report.proof.as_ref().unwrap().verified);
    assert_eq!(report.checks.attacks_total, 0);
    let evidence = bundle::create(input, Mode::Lean);
    assert!(bundle::verify(&evidence).0);
}

#[test]
fn spend_target_policy_rejects_qualified_names_and_representation_access() {
    for expression in [
        "decide (UInt64.toNat spent + UInt64.toNat amount ≤ UInt64.toNat limit)",
        "decide (_root_.UInt64.toNat spent ≤ 0)",
        "decide ((spent).toNat ≤ 0)",
        "decide (spent . toNat ≤ 0)",
        "decide (spent./- nested /- comment -/ -/toNat ≤ 0)",
        "let cast := UInt64.toNat; decide (cast spent ≤ 0)",
        "let n := Nat.add 1 2; decide (n ≤ 3)",
        "let n : Int := 0; decide (n ≤ 0)",
        "let bits := spent.toBitVec; bits == amount.toBitVec",
        "let bits : BitVec 64 := 0; bits == bits",
        "let n := spent.toFin.val; decide (n ≤ 0)",
        "let n : Fin 1 := 0; decide (n.val ≤ 0)",
        "let n : UInt128 := 0; n == n",
        "let n := spent.toUInt32; n == n",
        "let n := UInt64.size; decide (n ≤ n)",
        "decide (spent.1.1.1 + amount.1.1.1 ≤ limit.1.1.1)",
        "decide ((spent).1.1.1 ≤ 0)",
        "decide (spent . 1 . 1 . 1 ≤ 0)",
        "let ⟨⟨⟨n, _⟩⟩⟩ := spent; decide (n ≤ 0)",
        "match spent with | ⟨n⟩ => n == n",
        "by cases spent; exact true",
        "UInt64.casesOn spent (fun bits => bits == bits)",
        "UInt64.rec (fun bits => bits == bits) spent",
        "UInt64.recOn spent (fun bits => bits == bits)",
    ] {
        let report = judge_lean(&with_target(expression));
        assert_eq!(report.verdict, "reject", "{expression}");
        assert_eq!(
            report.reason_codes,
            [Reason::PolicyViolation],
            "{expression}"
        );
        assert_eq!(report.checks.attacks_total, 0, "{expression}");
    }
}

#[test]
fn spend_target_policy_rejects_scope_alias_and_syntax_definitions() {
    for directive in [
        "open Nat in",
        "open UInt64 in",
        "export UInt64 (toNat)",
        "abbrev cast := UInt64.toNat",
        "notation \"cast\" => UInt64.toNat",
        "local notation \"cast\" => UInt64.toNat",
        "infixl:65 \"plus\" => Nat.add",
        "macro \"allowed\" : term => `(true)",
        "macro_rules | `(allowed) => `(true)",
        "let cast := UInt64.«toNat»",
    ] {
        let mut input = with_target("true");
        let source = String::from_utf8(input.candidate["Target.lean"].clone()).unwrap();
        // Both command scope and expression scope must be checked before the fixed wrapper.
        for target in [
            format!("{directive}\n{source}"),
            source.replace("  true", &format!("  {directive}\n  true")),
        ] {
            input
                .candidate
                .insert("Target.lean".into(), target.into_bytes());
            let report = judge_lean(&input);
            assert_eq!(report.verdict, "reject", "{directive}");
            assert!(
                report.reason_codes.contains(&Reason::PolicyViolation),
                "{directive}"
            );
        }
    }
}

#[test]
fn elaborated_target_policy_rejects_inferred_arithmetic_and_coercions() {
    let original = fixture("after_honest");
    let source = String::from_utf8(original.candidate["Target.lean"].clone()).unwrap();
    for prefix in [
        "let wider := 18446744073709551616 + 1;",
        "let wider := -1;",
        "let wider := (2 ^ 64 + 1) == 0;",
        "let wider := (toString spent).length + (toString amount).length;",
        "let wider := if (toString spent).length ≤ (toString spent).length then true else false;",
        "let convert : Coe _ UInt64 := { coe := fun _ => limit }; let wider := convert.coe ((toString spent).length);",
        "letI : Coe ((fun {alpha : Type} (_ : alpha) => alpha) ((toString spent).length)) UInt64 := { coe := fun n => if n == (toString spent).length then limit else spent }; let wider : UInt64 := ↑((toString spent).length);",
    ] {
        // Keep the fixed tests and reference proof; policy inspection precedes proof checking.
        let mut input = original.clone();
        input.candidate.insert(
            "Target.lean".into(),
            source
                .replacen(" :=\n", &format!(" :=\n  {prefix}\n"), 1)
                .into_bytes(),
        );
        let report = judge_lean(&input);
        assert_eq!(report.verdict, "reject", "{prefix}");
        assert_eq!(report.reason_codes, [Reason::PolicyViolation], "{prefix}");
        assert!(
            report
                .policy_violations
                .iter()
                .any(|message| message.contains("elaborated"))
        );
        assert!(!report.proof.unwrap().verified);
        assert!(bundle::verify(&bundle::create(input, Mode::Lean)).0);
    }
}

#[test]
fn spend_attack_literals_are_normalized_across_numeric_bases() {
    let original = fixture("after_honest");
    let source = String::from_utf8(original.candidate["Target.lean"].clone()).unwrap();
    for (maximum, one) in [
        ("18446744073709551615", "1"),
        ("0xffffffffffffffff", "0x1"),
        ("0o1777777777777777777777", "0o1"),
        (
            "0b1111111111111111111111111111111111111111111111111111111111111111",
            "0b1",
        ),
    ] {
        let mut input = original.clone();
        input.candidate.insert("Target.lean".into(), source.replacen(
            " :=\n", &format!(" :=\n  let boundary : UInt64 := {maximum}\n  let same : UInt64 := {maximum}\n  let unit : UInt64 := {one}\n"), 1,
        ).into_bytes());
        assert!(
            judge_lean(&input)
                .reason_codes
                .contains(&Reason::PolicyViolation),
            "{maximum}"
        );
    }
}

#[test]
fn spend_target_policy_is_token_based_and_leaves_proofs_unrestricted() {
    let mut input = fixture("after_honest");
    input
        .candidate
        .get_mut("Target.lean")
        .unwrap()
        .extend_from_slice(
            b"\n-- toNat Nat Int open export abbrev notation macro\n/- toFin /- val -/ BitVec -/\n",
        );
    let source = String::from_utf8(input.candidate["Target.lean"].clone()).unwrap();
    input.candidate.insert(
        "Target.lean".into(),
        source
            .replacen(
                " :=\n",
                " :=\n  let toNat_is_not_a_cast := true\n  let text := \"toNat Nat open\"\n",
                1,
            )
            .into_bytes(),
    );
    let proof = String::from_utf8(input.candidate["Proof.lean"].clone()).unwrap();
    input.candidate.insert(
        "Proof.lean".into(),
        proof.replacen(" := by", " := by\n  have conversion (n : UInt64) : n.toNat = n.toNat := by rfl\n  have arithmetic (n : Nat) : n = n := by rfl", 1).into_bytes(),
    );
    let report = judge_lean(&input);
    assert_eq!(report.verdict, "accept");
    assert!(report.proof.unwrap().verified);
    let mut policy: serde_json::Value = serde_json::from_slice(&input.policy).unwrap();
    policy["target_forbidden_identifiers"]
        .as_array_mut()
        .unwrap()
        .push("decide".into());
    input.policy = serde_json::to_vec(&policy).unwrap();
    assert_eq!(judge_lean(&input).reason_codes, [Reason::PolicyViolation]);
}

#[test]
fn spend_target_restrictions_do_not_apply_to_path() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean");
    let mut input = load_lean_subject_bundle(&root, "after_honest", Subject::Path).unwrap();
    let source = String::from_utf8(input.candidate["Target.lean"].clone()).unwrap();
    input.candidate.insert(
        "Target.lean".into(),
        source
            .replacen(" :=\n", " :=\n  let n : Nat := (0 : UInt64).toNat\n", 1)
            .into_bytes(),
    );
    let mut policy: serde_json::Value = serde_json::from_slice(&input.policy).unwrap();
    let spend_policy: serde_json::Value =
        serde_json::from_slice(&fixture("after_honest").policy).unwrap();
    policy["target_forbidden_identifiers"] = spend_policy["target_forbidden_identifiers"].clone();
    input.policy = serde_json::to_vec(&policy).unwrap();
    let report = judge_lean(&input);
    assert_eq!(report.judge_version, "0.1.0/lean-path-7");
    assert_eq!(report.verdict, "accept");
    assert!(report.proof.unwrap().verified);
}

#[test]
fn spend_versions_have_expected_verdicts_and_counts() {
    for (version, reasons, attacks, regressions) in [
        ("before", vec![Reason::AttackStillWorks], 12, 16),
        ("after_honest", vec![], 0, 16),
        (
            "after_hardcode",
            vec![Reason::PolicyViolation, Reason::AttackStillWorks],
            11,
            16,
        ),
        ("after_cheat", vec![Reason::ProofCheated], 0, 16),
        ("after_reject_all", vec![Reason::RegressionFailed], 0, 7),
        ("after_saturate", vec![Reason::AttackStillWorks], 3, 16),
    ] {
        let input = fixture(version);
        assert_eq!(
            input.candidate["tests/regression.json"],
            input.baseline["tests/regression.json"]
        );
        let report = judge_lean(&input);
        assert_eq!(report.reason_codes, reasons, "{version}");
        assert_eq!(
            report.verdict,
            if version == "after_honest" {
                "accept"
            } else {
                "reject"
            }
        );
        assert_eq!(report.checks.attacks_total, 12);
        assert_eq!(report.checks.baseline_attacks_working, 12);
        assert_eq!(
            report.checks.candidate_attacks_working, attacks,
            "{version}"
        );
        assert_eq!(report.checks.regressions_total, 16);
        assert_eq!(report.checks.regressions_passed, regressions, "{version}");
        assert_eq!(report.proof.unwrap().verified, version == "after_honest");
    }
}

#[test]
fn saturation_at_maximum_limit_disagrees_with_the_specification() {
    let input = "18446744073709551615 18446744073709551615 1";
    assert!(u128::from(u64::MAX) + 1 > u128::from(u64::MAX));
    for (version, expected) in [("after_saturate", "true"), ("after_honest", "false")] {
        let bundle = fixture(version);
        assert_eq!(
            lean::run_for(Subject::Spend, &bundle.candidate["Target.lean"], &[input]).unwrap(),
            [Some(expected.into())]
        );
    }
}

#[test]
fn spend_bundles_replay_and_bind_the_subject_to_the_hash() {
    for version in [
        "after_honest",
        "after_hardcode",
        "after_cheat",
        "after_reject_all",
        "after_saturate",
    ] {
        let evidence = bundle::create(fixture(version), Mode::Lean);
        assert!(bundle::verify(&evidence).0, "{version}");
    }
    let mut evidence = bundle::create(fixture("after_honest"), Mode::Lean);
    let original = hash_inputs(&evidence.input);
    assert_eq!(original.0["subject"], sha256(b"spend"));
    evidence.input.subject = Subject::Path;
    assert_ne!(original.1, hash_inputs(&evidence.input).1);
    assert!(!bundle::verify(&evidence).0);
}

#[test]
fn spend_fixed_inputs_and_theorem_type_cannot_be_replaced() {
    let original = fixture("after_honest");
    assert_eq!(
        sha256(&original.candidate["SpendSpec.lean"]),
        Subject::Spend.spec_sha256()
    );
    for &(path, _) in Subject::Spend.trusted() {
        let mut bundle = original.clone();
        for tree in [&mut bundle.baseline, &mut bundle.candidate] {
            tree.get_mut(path)
                .unwrap()
                .extend_from_slice(b"\n-- modified\n");
        }
        bundle.policy = serde_json::to_vec(&serde_json::json!({
            "editable_paths": ["Target.lean", "Proof.lean", path],
            "immutable_paths": ["tests/regression.json"], "forbid_attack_literals": false
        }))
        .unwrap();
        assert_eq!(
            lean::inspect_for(Subject::Spend, &bundle.baseline, &bundle.candidate),
            Some(Reason::ProofCheated),
            "{path}"
        );
        assert_eq!(judge_lean(&bundle).verdict, "reject");
    }
    let mut bundle = original;
    bundle.candidate.insert(
        "Proof.lean".into(),
        b"import Target\ntheorem patch_sound : True := by trivial\n".to_vec(),
    );
    assert_eq!(judge_lean(&bundle).reason_codes, [Reason::ProofCheated]);
}

#[test]
fn forbidden_constructs_remain_rejected_for_both_subjects() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean");
    for subject in [Subject::Path, Subject::Spend] {
        let original = load_lean_subject_bundle(&root, "after_honest", subject).unwrap();
        for forbidden in [
            "sorry",
            "native_decide",
            "bv_decide",
            "decide +native",
            "@[implemented_by backdoor]",
            "@[extern backdoor]",
            "unsafe",
            "opaque backdoor : False",
            "axiom backdoor : False",
            "set_option debug.skipKernelTC true",
            "run_tac pure ()",
            "Lean.ofReduceBool",
            "«sorry»",
            "import Lean",
            "initialize pure ()",
        ] {
            let mut bundle = original.clone();
            bundle
                .candidate
                .get_mut("Proof.lean")
                .unwrap()
                .extend_from_slice(format!("\n{forbidden}\n").as_bytes());
            assert_eq!(
                lean::inspect_for(subject, &bundle.baseline, &bundle.candidate),
                Some(Reason::ProofCheated),
                "{subject:?}: {forbidden}"
            );
            assert_eq!(judge_lean(&bundle).verdict, "reject");
        }
    }
}

#[test]
fn general_tactics_and_core_lemma_names_are_usable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean");
    let mut bundle = load_lean_subject_bundle(&root, "after_honest", Subject::Path).unwrap();
    let proof = String::from_utf8(bundle.candidate["Proof.lean"].clone()).unwrap();
    let proof = proof.replacen(" := by", " := by\n  have arithmetic : 2 + 3 = 5 := by decide\n  have auxiliary (n : Nat) : n + 0 = n := by\n    induction n with\n    | zero => rfl\n    | succ n ih => rw [Nat.add_zero]\n  have bounded (n : Nat) : n ≤ n + 1 := by omega\n  have excluded (n : Nat) : n = 0 ∨ n ≠ 0 := by\n    by_cases hzero : n = 0\n    · exact Or.inl hzero\n    · exact Or.inr hzero", 1);
    bundle
        .candidate
        .insert("Proof.lean".into(), proof.into_bytes());
    assert_eq!(judge_lean(&bundle).verdict, "accept");
}

#[test]
fn a_change_outside_the_finite_spend_cases_still_fails_the_kernel_check() {
    let mut bundle = fixture("after_honest");
    let source = String::from_utf8(bundle.candidate["Target.lean"].clone()).unwrap();
    bundle.candidate.insert(
        "Target.lean".into(),
        source
            .replacen(
                " :=\n",
                " :=\n  if limit == 200 && spent == 0 && amount == 1 then false else\n",
                1,
            )
            .into_bytes(),
    );
    let report = judge_lean(&bundle);
    assert_eq!(report.checks.candidate_attacks_working, 0);
    assert_eq!(report.checks.regressions_passed, 16);
    assert_eq!(report.reason_codes, [Reason::ProofRejected]);
}

#[test]
fn malformed_spend_cases_and_changed_regressions_are_rejected() {
    for input in [
        "0 -1 1",
        "18446744073709551616 0 0",
        "0 0",
        "0 0 0",
        "0  1 2",
        "+0 18446744073709551615 1",
        "0 +18446744073709551615 1",
        "0 18446744073709551615 +1",
    ] {
        let mut bundle = fixture("after_honest");
        bundle.corpus = serde_json::to_vec(
            &serde_json::json!({"public": [input], "variants": ["0 18446744073709551615 1"]}),
        )
        .unwrap();
        assert_eq!(judge_lean(&bundle).verdict, "reject", "{input}");
    }
    let mut bundle = fixture("after_honest");
    bundle
        .candidate
        .insert("tests/regression.json".into(), b"[]".to_vec());
    let report = judge_lean(&bundle);
    assert_eq!(report.reason_codes, [Reason::PolicyViolation]);
    assert_eq!(report.checks.regressions_passed, 16);
}

#[test]
fn spend_cli_and_bundle_roundtrip_are_deterministic() {
    use std::io::Write;
    use std::process::Stdio;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean");
    let execute = || {
        Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
            .args([
                "--lean",
                "--subject",
                "spend",
                root.to_str().unwrap(),
                "after_honest",
            ])
            .output()
            .unwrap()
    };
    let first = execute();
    assert!(first.status.success());
    assert_eq!(first.stdout, execute().stdout);
    let mut child = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
        .args(["--lean", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&fixture("after_honest")).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(first.stdout, output.stdout);
    let mut child = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
        .args(["--lean", "--subject", "path", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&fixture("after_honest")).unwrap())
        .unwrap();
    let mismatch = child.wait_with_output().unwrap();
    assert_eq!(mismatch.status.code(), Some(2));
    assert!(mismatch.stdout.is_empty());
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("subject differs"));
}

#[test]
fn spend_proof_is_not_copied_into_other_fixtures_source_or_docs() {
    assert!(
        Command::new("python3")
            .arg("../scripts/check-spend-proof-leak.py")
            .arg("--self-test")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .unwrap()
            .success()
    );
}
