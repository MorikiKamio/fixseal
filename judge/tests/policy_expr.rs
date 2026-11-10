use fixseal_judge::{
    Reason, bundle, export_matches, input,
    lean::{self, Subject},
    policy_expr::BoolExpr,
    sha256, spend_boundary_grid,
};
use std::path::Path;

fn source(body: &str) -> Vec<u8> {
    format!("import SpendSpec\nnamespace Target\ndef authorize (limit spent amount : UInt64) : Bool :=\n {body}\nend Target\n").into_bytes()
}

#[test]
fn elaborated_exports_cover_operators_lets_and_propositions() {
    let grid = spend_boundary_grid();
    assert_eq!(grid.len(), 1000);
    let inputs: Vec<_> = grid.iter().map(String::as_str).collect();
    for body in [
        "decide (spent ≤ limit ∧ amount ≤ limit - spent)",
        "if spent < limit then !(spent == amount) && decide (amount ≤ limit - spent) else (spent == limit) || false",
        "let sum := if spent < amount then spent + amount else spent - amount; sum == limit",
        "if decide (spent < amount) then true else false",
        "decide (¬ (spent = amount) ∨ spent < limit)",
    ] {
        let execution = lean::run_export_for(Subject::Spend, &source(body), &inputs).unwrap();
        let json = execution.policy_expr.unwrap();
        let expr = BoolExpr::parse(&json).unwrap();
        assert_eq!(expr.canonical(), json);
        assert!(export_matches(&expr, &inputs, &execution.outputs), "{body}");
    }
}

#[test]
fn unsupported_elaborated_fragment_fails_closed() {
    assert_eq!(
        lean::run_export_for(
            Subject::Spend,
            &source("decide (spent * amount ≤ limit)"),
            &["1 1 1"]
        ),
        Err(Reason::PolicyViolation)
    );
}

#[test]
fn correspondence_detects_a_single_wrong_or_missing_result() {
    let expr = BoolExpr::vulnerable();
    let inputs = ["1000 600 18446744073709551116", "1000 0 1"];
    let mut outputs = vec![Some("true".into()), Some("true".into())];
    assert!(export_matches(&expr, &inputs, &outputs));
    outputs[1] = Some("false".into());
    assert!(!export_matches(&expr, &inputs, &outputs));
    assert!(!export_matches(&expr, &inputs, &outputs[..1]));
    assert_eq!(
        serde_json::to_string(&Reason::ExportMismatch).unwrap(),
        "\"EXPORT_MISMATCH\""
    );
}

#[test]
fn spend_report_summary_and_replay_bind_the_export() {
    let input = input::load_lean_subject_bundle(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean"),
        "after_honest",
        Subject::Spend,
    )
    .unwrap();
    let mut evidence = bundle::create(input, bundle::Mode::Lean);
    assert_eq!(evidence.report["verdict"], "accept");
    assert_eq!(evidence.report["export_checked_inputs"], 1028);
    let json = evidence.report["policy_expr"].as_str().unwrap();
    assert_eq!(
        evidence.report["policy_expr_sha256"],
        sha256(json.as_bytes())
    );
    let summary: serde_json::Value =
        serde_json::from_str(&bundle::summary(&evidence.report)).unwrap();
    assert_eq!(summary["subject"], "spend");
    assert_eq!(summary["spec_sha256"], Subject::Spend.spec_sha256());
    assert_eq!(summary["policy_expr"], evidence.report["policy_expr"]);
    assert!(bundle::verify(&evidence).0);
    evidence.report["policy_expr"] = "{\"op\":\"true\"}".into();
    evidence.report["policy_expr_sha256"] = sha256(b"{\"op\":\"true\"}").into();
    let (valid, lines) = bundle::verify(&evidence);
    assert!(!valid);
    assert!(
        lines
            .iter()
            .any(|s| s == "MISMATCH re-exported policy expression and SHA-256")
    );
}
