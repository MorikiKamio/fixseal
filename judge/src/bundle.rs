use crate::input::Bundle;
use crate::{hash_inputs, judge, judge_lean, lean, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Vm,
    Lean,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainRecord {
    pub bundle_hash: String,
    pub caller: String,
    pub block_timestamp_ms: String,
    pub summary: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub format: String,
    pub mode: Mode,
    pub input: Bundle,
    pub report: Value,
    pub report_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<ChainRecord>,
}

pub fn summary(report: &Value) -> String {
    let mut fields = std::collections::BTreeMap::from([
        ("judge_version", report["judge_version"].clone()),
        ("inputs_sha256", report["inputs_sha256"].clone()),
        ("verdict", report["verdict"].clone()),
    ]);
    if report["judge_version"] == "0.1.0/lean-spend-4" {
        fields.insert("subject", json!("spend"));
        fields.insert(
            "spec_sha256",
            report["proof"]["approved_spec_sha256"].clone(),
        );
        fields.insert("policy_expr", report["policy_expr"].clone());
        fields.insert("policy_expr_sha256", report["policy_expr_sha256"].clone());
    }
    serde_json::to_string(&fields).expect("serializable summary")
}

pub fn create(input: Bundle, mode: Mode) -> Evidence {
    let report = match mode {
        Mode::Vm => judge(&input),
        Mode::Lean => judge_lean(&input),
    };
    Evidence {
        format: "fixseal-bundle-v1".into(),
        mode,
        report_sha256: sha256(&report.to_json()),
        input,
        report: serde_json::to_value(report).expect("serializable report"),
        chain: None,
    }
}

fn check(lines: &mut Vec<String>, label: &str, matches: bool) -> bool {
    lines.push(format!(
        "{} {label}",
        if matches { "OK" } else { "MISMATCH" }
    ));
    matches
}

// The embedded record is an offline readback, not an authenticated RPC response.
pub fn verify(evidence: &Evidence) -> (bool, Vec<String>) {
    let mut lines = vec!["simulated environment / offline bundle verification".into()];
    let mut valid = check(
        &mut lines,
        "bundle format",
        evidence.format == "fixseal-bundle-v1",
    );
    let (hashes, inputs_sha256) = hash_inputs(&evidence.input);
    valid &= check(
        &mut lines,
        "individual input SHA-256",
        evidence.report["input_hashes"] == json!(hashes),
    );
    valid &= check(
        &mut lines,
        "combined input SHA-256",
        evidence.report["inputs_sha256"] == inputs_sha256,
    );
    if !valid {
        return (false, lines);
    }
    let replay = match evidence.mode {
        Mode::Vm => judge(&evidence.input),
        Mode::Lean => judge_lean(&evidence.input),
    };
    valid &= check(
        &mut lines,
        "replayed verdict",
        evidence.report["verdict"] == replay.verdict,
    );
    if let Mode::Lean = evidence.mode {
        let proof = lean::inspect_for(
            evidence.input.subject,
            &evidence.input.baseline,
            &evidence.input.candidate,
        )
        .map_or_else(
            || lean::verify_for(evidence.input.subject, &evidence.input.candidate),
            Err,
        );
        lines.push(match &proof {
            Ok(()) => "OK Lean: lake build + axiom audit + forbidden syntax inspection".into(),
            Err(reason) => format!("Lean proof rejected: {}", json!(reason)),
        });
        // Rejected submissions can be verified; an accepted proof must recheck successfully.
        valid &= check(
            &mut lines,
            "Lean proof status",
            replay
                .proof
                .as_ref()
                .is_some_and(|p| !p.verified || proof.is_ok()),
        );
    }
    if evidence.input.subject == lean::Subject::Spend {
        valid &= check(
            &mut lines,
            "re-exported policy expression and SHA-256",
            evidence.report["policy_expr"] == json!(replay.policy_expr)
                && evidence.report["policy_expr_sha256"] == json!(replay.policy_expr_sha256),
        );
    }
    valid &= check(
        &mut lines,
        "complete replayed report (version, reasons, checks, spec)",
        evidence.report == serde_json::to_value(&replay).expect("serializable report"),
    );
    let report_sha256 = sha256(&replay.to_json());
    valid &= check(
        &mut lines,
        "result SHA-256",
        evidence.report_sha256 == report_sha256,
    );
    if let Some(chain) = &evidence.chain {
        valid &= check(
            &mut lines,
            "chain get_fix_proof.bundle_hash",
            chain.bundle_hash == report_sha256,
        );
        valid &= check(
            &mut lines,
            "chain get_fix_proof.summary (verdict, version, inputs)",
            chain.summary == summary(&evidence.report),
        );
        lines.push(format!(
            "offline readback: caller={}, block_timestamp_ms={}",
            chain.caller, chain.block_timestamp_ms
        ));
    } else {
        lines.push("chain readback: not supplied".into());
    }
    (valid, lines)
}
