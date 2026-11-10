pub mod bundle;
pub mod input;
pub mod lean;
#[path = "../../shared/policy_expr.rs"]
pub mod policy_expr;
pub mod vm;

use input::{Bundle, valid_path};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use vm::{Program, decode_percent, normalize, within_root};

pub const JUDGE_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "/path-vm-1");
const SOURCE: &str = "path.fn";
const REGRESSION: &str = "tests/regression.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    public: Vec<String>,
    variants: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Regression {
    input: String,
    expected: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    editable_paths: Vec<String>,
    immutable_paths: Vec<String>,
    forbid_attack_literals: bool,
    #[serde(default)]
    target_forbidden_identifiers: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum Reason {
    #[serde(rename = "EXPORT_MISMATCH")]
    ExportMismatch,
    #[serde(rename = "VULN_NOT_REPRODUCED")]
    VulnNotReproduced,
    #[serde(rename = "ATTACK_STILL_WORKS")]
    AttackStillWorks,
    #[serde(rename = "REGRESSION_FAILED")]
    RegressionFailed,
    #[serde(rename = "POLICY_VIOLATION")]
    PolicyViolation,
    #[serde(rename = "PROOF_MISSING")]
    ProofMissing,
    #[serde(rename = "PROOF_REJECTED")]
    ProofRejected,
    #[serde(rename = "PROOF_TIMEOUT")]
    ProofTimeout,
    #[serde(rename = "PROOF_CHEATED")]
    ProofCheated,
}

#[derive(Default, Serialize)]
pub struct Checks {
    pub attacks_total: usize,
    pub baseline_attacks_working: usize,
    pub candidate_attacks_working: usize,
    pub regressions_total: usize,
    pub regressions_passed: usize,
}

#[derive(Serialize)]
pub struct ProofChecks {
    pub verified: bool,
    pub approved_spec_sha256: &'static str,
}

#[derive(Serialize)]
pub struct Report {
    pub judge_version: &'static str,
    pub validation_scope: &'static str,
    pub verdict: &'static str,
    pub reason_codes: Vec<Reason>,
    pub input_hashes: BTreeMap<String, String>,
    pub inputs_sha256: String,
    pub checks: Checks,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<ProofChecks>,
    pub policy_violations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_expr_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_checked_inputs: Option<usize>,
}

impl Report {
    pub fn to_json(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(self).expect("report serialization cannot fail");
        bytes.push(b'\n');
        bytes
    }

    fn policy_violation(&mut self, message: &str) {
        if !self.reason_codes.contains(&Reason::PolicyViolation) {
            self.reason_codes.push(Reason::PolicyViolation);
        }
        self.policy_violations.push(message.into());
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn hash_inputs(bundle: &Bundle) -> (BTreeMap<String, String>, String) {
    let mut files = BTreeMap::new();
    files.insert("version".into(), bundle.version.as_bytes());
    if !bundle.subject.is_path() {
        files.insert("subject".into(), bundle.subject.name().as_bytes());
    }
    files.insert("corpus.json".into(), bundle.corpus.as_slice());
    files.insert("policy.json".into(), bundle.policy.as_slice());
    for (prefix, tree) in [
        ("baseline", &bundle.baseline),
        ("candidate", &bundle.candidate),
    ] {
        for (path, bytes) in tree {
            files.insert(format!("{prefix}/{path}"), bytes.as_slice());
        }
    }
    let mut all = Sha256::new();
    all.update(b"fixseal-inputs-v1\0");
    let hashes = files
        .into_iter()
        .map(|(path, bytes)| {
            all.update((path.len() as u64).to_be_bytes());
            all.update(path.as_bytes());
            all.update((bytes.len() as u64).to_be_bytes());
            all.update(bytes);
            (path, sha256(bytes))
        })
        .collect();
    (hashes, format!("{:x}", all.finalize()))
}

pub fn judge(bundle: &Bundle) -> Report {
    judge_with_mode(bundle, false)
}

pub fn judge_lean(bundle: &Bundle) -> Report {
    judge_with_mode(bundle, true)
}

fn judge_with_mode(bundle: &Bundle, lean_mode: bool) -> Report {
    let (input_hashes, inputs_sha256) = hash_inputs(bundle);
    let mut report = Report {
        judge_version: if lean_mode {
            match bundle.subject {
                lean::Subject::Path => "0.1.0/lean-path-7",
                lean::Subject::Spend => "0.1.0/lean-spend-4",
            }
        } else {
            JUDGE_VERSION
        },
        validation_scope: "仮想空間での検証",
        verdict: "reject",
        reason_codes: Vec::new(),
        input_hashes,
        inputs_sha256,
        checks: Checks::default(),
        proof: lean_mode.then_some(ProofChecks {
            verified: false,
            approved_spec_sha256: bundle.subject.spec_sha256(),
        }),
        policy_violations: Vec::new(),
        policy_expr: None,
        policy_expr_sha256: None,
        export_checked_inputs: None,
    };
    let proof_issue = lean_mode
        .then(|| lean::inspect_for(bundle.subject, &bundle.baseline, &bundle.candidate))
        .flatten();
    if proof_issue == Some(Reason::ProofCheated) {
        report.reason_codes.push(Reason::ProofCheated);
    }
    if lean_mode
        && bundle.subject == lean::Subject::Spend
        && let Ok(policy) = serde_json::from_slice::<Policy>(&bundle.policy)
        && let Some(source) = bundle.candidate.get("Target.lean")
        && let Some(violation) =
            lean::target_policy_violation(source, &policy.target_forbidden_identifiers)
    {
        report.policy_violation(&violation);
        return report;
    }
    evaluate(bundle, &mut report, lean_mode);
    if lean_mode && report.reason_codes.is_empty() {
        match proof_issue.map_or_else(|| lean::verify_for(bundle.subject, &bundle.candidate), Err) {
            Ok(()) => report.proof.as_mut().expect("Lean checks exist").verified = true,
            Err(reason) => report.reason_codes.push(reason),
        }
    }
    if report.reason_codes.is_empty() {
        report.verdict = "accept";
    }
    report
}

fn evaluate(bundle: &Bundle, report: &mut Report, lean_mode: bool) {
    let spend = bundle.subject == lean::Subject::Spend;
    if spend && !lean_mode {
        report.policy_violation("spend requires Lean mode");
        return;
    }
    let source_path = if lean_mode { "Target.lean" } else { SOURCE };
    let Ok(corpus) = serde_json::from_slice::<Corpus>(&bundle.corpus) else {
        report.policy_violation("invalid corpus");
        return;
    };
    let Ok(policy) = serde_json::from_slice::<Policy>(&bundle.policy) else {
        report.policy_violation("invalid policy");
        return;
    };
    if corpus.public.is_empty()
        || corpus.variants.is_empty()
        || corpus
            .public
            .iter()
            .chain(&corpus.variants)
            .any(String::is_empty)
    {
        report.policy_violation("public and variant attacks must be nonempty");
        return;
    }
    if !valid_path(&bundle.version)
        || bundle.version.contains('/')
        || bundle
            .baseline
            .keys()
            .chain(bundle.candidate.keys())
            .any(|p| !valid_path(p))
        || policy
            .editable_paths
            .iter()
            .chain(&policy.immutable_paths)
            .any(|p| !valid_path(p))
        || !policy.editable_paths.iter().any(|p| p == source_path)
        || !policy.immutable_paths.iter().any(|p| p == REGRESSION)
        || policy
            .editable_paths
            .iter()
            .any(|p| policy.immutable_paths.contains(p))
    {
        report.policy_violation("invalid paths or unprotected regression tests");
        return;
    }
    let paths: BTreeSet<_> = bundle
        .baseline
        .keys()
        .chain(bundle.candidate.keys())
        .collect();
    for path in paths {
        if bundle.baseline.get(path) != bundle.candidate.get(path)
            && (!policy.editable_paths.contains(path) || policy.immutable_paths.contains(path))
        {
            report.policy_violation(&format!("forbidden change: {path}"));
        }
    }
    for path in &policy.immutable_paths {
        if !bundle.baseline.contains_key(path) || !bundle.candidate.contains_key(path) {
            report.policy_violation(&format!("missing immutable input: {path}"));
        }
    }
    let Some(source) = bundle.candidate.get(source_path) else {
        report.policy_violation("missing candidate source");
        return;
    };
    let attacks: Vec<_> = corpus
        .public
        .iter()
        .chain(&corpus.variants)
        .map(String::as_str)
        .collect();
    if spend
        && attacks.iter().any(|input| {
            spend_request(input)
                .is_none_or(|(l, s, a)| u128::from(s) + u128::from(a) <= u128::from(l))
        })
    {
        report.policy_violation("invalid spend attack");
        return;
    }
    if policy.forbid_attack_literals {
        for path in &policy.editable_paths {
            if let Some(bytes) = bundle.candidate.get(path) {
                let text = String::from_utf8_lossy(bytes);
                if attacks.iter().any(|attack| {
                    text.contains(attack)
                        || decode_percent(attack).is_some_and(|decoded| text.contains(&decoded))
                }) || (lean_mode && !spend && lean::contains_attack_literal(bytes, &attacks))
                    || (spend && lean::contains_spend_literal(bytes, &attacks))
                {
                    report.policy_violation(&format!("attack literal in: {path}"));
                }
            }
        }
    }
    // The baseline tests are authoritative even when the candidate weakens its copy.
    let tests = bundle
        .baseline
        .get(REGRESSION)
        .and_then(|bytes| serde_json::from_slice::<Vec<Regression>>(bytes).ok());
    if spend
        && tests.as_ref().is_some_and(|tests| {
            tests.iter().any(|test| {
                spend_request(&test.input).is_none_or(|(l, s, a)| {
                    test.expected.as_deref()
                        != Some(if u128::from(s) + u128::from(a) <= u128::from(l) {
                            "true"
                        } else {
                            "false"
                        })
                })
            })
        })
    {
        report.policy_violation("invalid spend regression");
        return;
    }
    let mut inputs: Vec<&str> = attacks.clone();
    if let Some(tests) = &tests {
        inputs.extend(tests.iter().map(|test| test.input.as_str()));
    }
    let grid = if spend {
        spend_boundary_grid()
    } else {
        Vec::new()
    };
    inputs.extend(grid.iter().map(String::as_str));
    let execute = |source: Option<&Vec<u8>>| -> Result<Vec<Option<String>>, Reason> {
        let source = source.ok_or(Reason::PolicyViolation)?;
        if lean_mode {
            lean::run_for(bundle.subject, source, &inputs).and_then(|outputs| {
                if spend
                    && outputs
                        .iter()
                        .any(|value| !matches!(value.as_deref(), Some("true" | "false")))
                {
                    Err(Reason::ProofRejected)
                } else {
                    Ok(outputs)
                }
            })
        } else {
            let program = Program::parse(source).map_err(|_| Reason::PolicyViolation)?;
            Ok(inputs.iter().map(|input| program.run(input)).collect())
        }
    };
    let baseline = execute(bundle.baseline.get(source_path));
    let candidate = execute(Some(source));
    if spend && let Ok(outputs) = &candidate {
        // Reuses this execution's cached elaborated export, never parses Target source as an AST.
        if let Ok(execution) = lean::run_export_for(bundle.subject, source, &inputs)
            && let Some(json) = execution.policy_expr
        {
            let expr = policy_expr::BoolExpr::parse(&json).expect("validated exported expression");
            report.policy_expr_sha256 = Some(sha256(json.as_bytes()));
            report.policy_expr = Some(json);
            report.export_checked_inputs = Some(inputs.len());
            if !export_matches(&expr, &inputs, outputs) {
                report.reason_codes.push(Reason::ExportMismatch);
            }
        } else {
            report.policy_violation("missing elaborated policy expression");
        }
    }
    let attack_count = |outputs: &[Option<String>]| {
        outputs[..attacks.len()]
            .iter()
            .filter(|output| {
                if spend {
                    output.as_deref() == Some("true")
                } else {
                    output
                        .as_ref()
                        .is_some_and(|path| !within_root(&normalize(path)))
                }
            })
            .count()
    };
    report.checks.attacks_total = attacks.len();
    if let Ok(outputs) = &baseline {
        report.checks.baseline_attacks_working = attack_count(outputs);
    } else if lean_mode {
        let reason = if baseline == Err(Reason::ProofTimeout) {
            Reason::ProofTimeout
        } else {
            Reason::ProofRejected
        };
        report.reason_codes.push(reason);
    }
    if report.checks.baseline_attacks_working != attacks.len() {
        report.reason_codes.push(Reason::VulnNotReproduced);
    }
    if candidate == Err(Reason::PolicyViolation) {
        report.policy_violation("unsupported elaborated Target.authorize policy expression");
        return;
    }
    match &candidate {
        Ok(outputs) => {
            report.checks.candidate_attacks_working = attack_count(outputs);
            if report.checks.candidate_attacks_working > 0 {
                report.reason_codes.push(Reason::AttackStillWorks);
            }
        }
        Err(reason) if lean_mode => {
            if !report.reason_codes.contains(reason) {
                report.reason_codes.push(reason.clone());
            }
        }
        Err(_) => report.policy_violation("invalid candidate program"),
    }
    match tests {
        Some(tests) if !tests.is_empty() => {
            report.checks.regressions_total = tests.len();
            if let Ok(outputs) = &candidate {
                report.checks.regressions_passed = tests
                    .iter()
                    .zip(&outputs[attacks.len()..])
                    .filter(|(test, result)| test.expected == **result)
                    .count();
            }
            if report.checks.regressions_passed != tests.len() {
                report.reason_codes.push(Reason::RegressionFailed);
            }
        }
        _ => report.policy_violation("missing, empty or invalid baseline regression tests"),
    }
}

fn spend_request(input: &str) -> Option<(u64, u64, u64)> {
    if input
        .split(' ')
        .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let values = input
        .split(' ')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    match values.as_slice() {
        &[l, s, a] => Some((l, s, a)),
        _ => None,
    }
}

// All triples, including equalities on either side of each fixed-width boundary.
pub fn spend_boundary_grid() -> Vec<String> {
    let values = [
        0,
        1,
        (1u64 << 32) - 1,
        1u64 << 32,
        (1u64 << 32) + 1,
        (1u64 << 63) - 1,
        1u64 << 63,
        (1u64 << 63) + 1,
        u64::MAX - 1,
        u64::MAX,
    ];
    let mut inputs = Vec::new();
    for limit in values {
        for spent in values {
            for amount in values {
                inputs.push(format!("{limit} {spent} {amount}"));
            }
        }
    }
    inputs
}

pub fn export_matches(
    expr: &policy_expr::BoolExpr,
    inputs: &[&str],
    outputs: &[Option<String>],
) -> bool {
    inputs.len() == outputs.len()
        && inputs.iter().zip(outputs).all(|(input, output)| {
            spend_request(input).is_some_and(|(l, s, a)| {
                output.as_deref() == Some(if expr.eval(l, s, a) { "true" } else { "false" })
            })
        })
}
