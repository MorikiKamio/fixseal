use fixseal_judge::bundle::{self, Mode};
use fixseal_judge::input::load_lean_subject_bundle;
use fixseal_judge::lean::Subject;
use fixseal_judge::{Reason, judge_lean};
use std::path::Path;

fn check_attempts(agent: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for attempt in [1, 2] {
        let mut input =
            load_lean_subject_bundle(&root.join("lean"), "before", Subject::Spend).unwrap();
        input.version = format!("fx3_{agent}_attempt_{attempt}");
        let directory = root.join(format!("examples/fixers/spend/{agent}/attempt-{attempt}"));
        for name in ["Target.lean", "Proof.lean"] {
            input
                .candidate
                .insert(name.into(), std::fs::read(directory.join(name)).unwrap());
        }
        assert_eq!(
            input.candidate["tests/regression.json"],
            input.baseline["tests/regression.json"]
        );
        let report = judge_lean(&input);
        assert_eq!(
            report.verdict,
            if attempt == 1 { "reject" } else { "accept" }
        );
        assert_eq!(
            report.reason_codes,
            if attempt == 1 {
                vec![Reason::ProofRejected]
            } else {
                vec![]
            }
        );
        assert!(report.policy_violations.is_empty());
        assert_eq!(report.checks.baseline_attacks_working, 12);
        assert_eq!(report.checks.candidate_attacks_working, 0);
        assert_eq!(report.checks.attacks_total, 12);
        assert_eq!(report.checks.regressions_passed, 16);
        assert_eq!(report.checks.regressions_total, 16);
        assert_eq!(report.proof.as_ref().unwrap().verified, attempt == 2);
        let evidence = bundle::create(input, Mode::Lean);
        assert!(bundle::verify(&evidence).0);
    }
}

#[test]
fn devin_recorded_attempts_replay() {
    check_attempts("devin");
}

#[test]
fn codex_recorded_attempts_replay() {
    check_attempts("codex");
}

#[test]
fn nearai_recorded_final_replays() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut input = load_lean_subject_bundle(&root.join("lean"), "before", Subject::Spend).unwrap();
    input.version = "nx1_nearai_final".into();
    let directory = root.join("examples/fixers/spend/nearai-deepseek-v4.1-flash/final");
    for name in ["Target.lean", "Proof.lean"] {
        input
            .candidate
            .insert(name.into(), std::fs::read(directory.join(name)).unwrap());
    }
    assert_eq!(
        input.candidate["tests/regression.json"],
        input.baseline["tests/regression.json"]
    );
    let evidence = bundle::create(input, Mode::Lean);
    let report = &evidence.report;
    assert_eq!(report["verdict"], "accept");
    assert_eq!(report["reason_codes"], serde_json::json!([]));
    assert_eq!(report["policy_violations"], serde_json::json!([]));
    assert_eq!(report["checks"]["baseline_attacks_working"], 12);
    assert_eq!(report["checks"]["candidate_attacks_working"], 0);
    assert_eq!(report["checks"]["attacks_total"], 12);
    assert_eq!(report["checks"]["regressions_passed"], 16);
    assert_eq!(report["checks"]["regressions_total"], 16);
    assert_eq!(report["proof"]["verified"], true);
    assert_eq!(report["export_checked_inputs"], 1028);
    assert!(bundle::verify(&evidence).0);
}
