use fixseal_judge::{
    Reason, bundle, export_matches, input,
    lean::{self, Subject},
    policy_expr::BoolExpr,
    sha256, spend_boundary_grid,
};
use std::path::Path;

fn spend_export_inputs() -> Vec<String> {
    // Same order as the judge's export check: attacks, baseline regressions, grid.
    let spend = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/targets/spend");
    let corpus: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(spend.join("corpus.json")).unwrap()).unwrap();
    let mut inputs: Vec<String> = corpus["public"]
        .as_array()
        .unwrap()
        .iter()
        .chain(corpus["variants"].as_array().unwrap())
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let tests: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(spend.join("before/tests/regression.json")).unwrap(),
    )
    .unwrap();
    inputs.extend(
        tests
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["input"].as_str().unwrap().to_string()),
    );
    inputs.extend(spend_boundary_grid());
    inputs
}

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

// The viewer's JavaScript evaluator is tested against these vectors by
// viewer/policy.test.mjs; this test keeps the file equal to Rust results.
#[test]
fn viewer_policy_vectors_match_rust_evaluator() {
    let inputs = spend_export_inputs();
    assert_eq!(inputs.len(), 1028);
    // The spend-4 accepted export, the vulnerable baseline and one
    // expression covering every remaining operator.
    let accepted = BoolExpr::parse(
        r#"{"op":"and","args":[{"op":"le","args":[{"op":"amount"},{"op":"limit"}]},{"op":"le","args":[{"op":"spent"},{"op":"sub","args":[{"op":"limit"},{"op":"amount"}]}]}]}"#,
    )
    .unwrap();
    let all_ops = BoolExpr::parse(
        r#"{"op":"and","args":[{"op":"if","args":[{"op":"and","args":[{"op":"lt","args":[{"op":"spent"},{"op":"limit"}]},{"op":"not","args":{"op":"eq","args":[{"op":"spent"},{"op":"amount"}]}}]},{"op":"le","args":[{"op":"if","args":[{"op":"lt","args":[{"op":"spent"},{"op":"amount"}]},{"op":"add","args":[{"op":"spent"},{"op":"const","args":"18446744073709551615"}]},{"op":"sub","args":[{"op":"amount"},{"op":"spent"}]}]},{"op":"limit"}]},{"op":"or","args":[{"op":"false"},{"op":"eq","args":[{"op":"spent"},{"op":"limit"}]}]}]},{"op":"true"}]}"#,
    )
    .unwrap();
    let triple = |input: &str| {
        let parts: Vec<u64> = input.split(' ').map(|p| p.parse().unwrap()).collect();
        (parts[0], parts[1], parts[2])
    };
    let cases: Vec<serde_json::Value> = [
        ("vulnerable", BoolExpr::vulnerable()),
        ("accepted", accepted),
        ("all_operators", all_ops),
    ]
    .iter()
    .map(|(name, expr)| {
        let mut bits = vec![0u8; inputs.len().div_ceil(8)];
        for (i, input) in inputs.iter().enumerate() {
            let (limit, spent, amount) = triple(input);
            if expr.eval(limit, spent, amount) {
                bits[i / 8] |= 1 << (i % 8);
            }
        }
        let json = expr.canonical();
        serde_json::json!({
            "name": name,
            "expr": json,
            "expr_sha256": sha256(json.as_bytes()),
            "expected": bits.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        })
    })
    .collect();
    let vector = serde_json::json!({
        "format": "fixseal-policy-vectors-v1",
        "note": "expected is a little-endian bitset over inputs, from the Rust evaluator",
        "inputs": inputs,
        "cases": cases,
    });
    let text = serde_json::to_string(&vector).unwrap() + "\n";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../viewer/policy-vectors.json");
    if std::env::var_os("FIXSEAL_WRITE_VECTORS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        text,
        "regenerate: FIXSEAL_WRITE_VECTORS=1 cargo test --test policy_expr viewer_policy_vectors"
    );
}
