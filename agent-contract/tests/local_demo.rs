mod helpers;

use helpers::*;
use near_sandbox::{Sandbox, SandboxConfig};
use serde_json::{Value, json};
use shade_attestation::attestation::create_mock_dstack_attestation;
use shade_contract_template::FixProof;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

type Error = Box<dyn std::error::Error + Send + Sync>;

fn proof_summary(report: &Value) -> Result<String, serde_json::Error> {
    // near-api enables serde_json/preserve_order; the judge uses sorted object keys.
    let mut fields = BTreeMap::from([
        ("inputs_sha256", report["inputs_sha256"].clone()),
        ("judge_version", report["judge_version"].clone()),
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
    serde_json::to_string(&fields)
}

#[test]
fn summary_uses_the_judge_wire_format() {
    let report = json!({"verdict": "reject", "judge_version": "v1", "inputs_sha256": "ab"});
    assert_eq!(
        proof_summary(&report).unwrap(),
        r#"{"inputs_sha256":"ab","judge_version":"v1","verdict":"reject"}"#
    );
}

fn judge_command(args: &[&str]) -> Result<std::process::Output, Error> {
    let binary = std::env::var("FIXSEAL_JUDGE_BIN")?;
    let output = Command::new(binary).args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "fixseal-judge failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output)
}

#[tokio::test]
#[ignore = "run through scripts/demo.sh with preinstalled sandbox and wasm"]
async fn local_fix_proof_demo() -> Result<(), Error> {
    let binary = PathBuf::from(
        std::env::var("NEAR_SANDBOX_BIN_PATH")
            .map_err(|_| "NEAR_SANDBOX_BIN_PATH is required; automatic downloads are disabled")?,
    );
    if !binary.is_file() || !PathBuf::from(CONTRACT_WASM_PATH).is_file() {
        return Err(
            "Missing preinstalled sandbox binary or contract wasm; see docs/BLOCKED.md".into(),
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()?;
    let subject = std::env::var("FIXSEAL_DEMO_SUBJECT").unwrap_or_else(|_| "path".into());
    let versions = match subject.as_str() {
        "path" => vec![
            ("after_honest", "accept", json!([])),
            (
                "after_hardcode",
                "reject",
                json!(["POLICY_VIOLATION", "ATTACK_STILL_WORKS"]),
            ),
            ("after_weaken_test", "reject", json!(["POLICY_VIOLATION"])),
            ("after_cheat", "reject", json!(["PROOF_CHEATED"])),
            ("after_reject_all", "reject", json!(["REGRESSION_FAILED"])),
        ],
        "spend" => vec![
            ("after_honest", "accept", json!([])),
            (
                "after_hardcode",
                "reject",
                json!(["POLICY_VIOLATION", "ATTACK_STILL_WORKS"]),
            ),
            ("after_cheat", "reject", json!(["PROOF_CHEATED"])),
            ("after_reject_all", "reject", json!(["REGRESSION_FAILED"])),
            ("after_saturate", "reject", json!(["ATTACK_STILL_WORKS"])),
        ],
        _ => return Err("FIXSEAL_DEMO_SUBJECT must be path or spend".into()),
    };
    let lean_root = root.join("lean");
    let out_dir = root.join("demo-artifacts").join(&subject);
    std::fs::create_dir_all(&out_dir)?;
    let mut bundles = Vec::new();
    for (version, verdict, reasons) in versions {
        println!("Judging {subject}/{version}");
        let output = judge_command(&[
            "bundle",
            "--lean",
            "--subject",
            &subject,
            lean_root.to_str().unwrap(),
            version,
        ])?;
        let evidence: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(evidence["report"]["verdict"], verdict, "{version}");
        assert_eq!(evidence["report"]["reason_codes"], reasons, "{version}");
        let checks = &evidence["report"]["checks"];
        println!(
            "{version}: {verdict}; attacks={}/{}; regressions={}/{}; reasons={reasons}; SHA-256={}",
            checks["candidate_attacks_working"],
            checks["attacks_total"],
            checks["regressions_passed"],
            checks["regressions_total"],
            evidence["report_sha256"].as_str().unwrap()
        );
        std::fs::write(
            out_dir.join(format!("{version}.json")),
            serde_json::to_vec(&evidence)?,
        )?;
        bundles.push((version, evidence));
    }

    // No peers, telemetry, or remote RPC fallback. All communication stays on loopback.
    let config = SandboxConfig {
        additional_config: Some(json!({
            "telemetry": {"endpoints": []},
            "network": {"boot_nodes": "", "skip_sync_wait": true}
        })),
        ..Default::default()
    };
    let sandbox = Sandbox::start_sandbox_with_config(config).await
        .map_err(|error| format!("Local sandbox could not start: {error}; check binary, loopback permissions, and docs/BLOCKED.md"))?;
    println!("Sandbox started on loopback");
    let mut network = create_network_config(&sandbox);
    network.faucet_url = None;
    network.meta_transaction_relayer_url = None;
    network.fastnear_url = None;
    let (owner_id, owner_signer) = setup_genesis_account().await;
    let contract_id = deploy_contract_default(&network, &owner_id, &owner_signer).await?;
    call_transaction(
        &contract_id,
        "update_attestation_expiration_time",
        json!({"attestation_expiration_time_ms": "86400000"}),
        &owner_id,
        &owner_signer,
        &network,
        None,
    )
    .await?
    .assert_success();
    let (agent_id, agent_signer) =
        create_user_account(&network, &owner_id, &owner_signer, "fixseal-agent").await?;
    call_transaction(
        &contract_id,
        "whitelist_agent_for_local",
        json!({"account_id": agent_id}),
        &owner_id,
        &owner_signer,
        &network,
        None,
    )
    .await?
    .assert_success();
    call_transaction(
        &contract_id,
        "register_agent",
        json!({"attestation": create_mock_dstack_attestation()}),
        &agent_id,
        &agent_signer,
        &network,
        Some(DEPOSIT_005_NEAR),
    )
    .await?
    .assert_success();
    println!("Contract deployed; local agent whitelisted and registered (mock attestation)");

    for (version, evidence) in &mut bundles {
        let hash = evidence["report_sha256"]
            .as_str()
            .ok_or("missing result hash")?
            .to_string();
        let report = &evidence["report"];
        let summary = proof_summary(report)?;
        call_transaction(
            &contract_id,
            "submit_fix_proof",
            json!({"bundle_hash": hash, "summary": summary}),
            &agent_id,
            &agent_signer,
            &network,
            None,
        )
        .await?
        .assert_success();
        let readback = call_view::<Option<FixProof>>(
            &contract_id,
            "get_fix_proof",
            json!({"bundle_hash": hash}),
            &network,
        )
        .await?;
        let record = readback.data.ok_or("get_fix_proof returned no record")?;
        assert_eq!(record.bundle_hash, hash);
        assert_eq!(record.summary, summary);
        assert_eq!(record.caller.to_string(), agent_id.to_string());
        evidence["chain"] = serde_json::to_value(record)?;
        let path = out_dir.join(format!("{version}.json"));
        std::fs::write(&path, serde_json::to_vec(evidence)?)?;
        let verified = judge_command(&["verify", path.to_str().unwrap()])?;
        println!("{version}: submit_fix_proof → get_fix_proof → verify");
        print!("{}", String::from_utf8_lossy(&verified.stdout));
    }
    println!(
        "\nsimulated environment, synthetic data — {subject} / local sandbox / mock attestation"
    );
    println!(
        "| version | verdict | attacks working | regressions passed | reasons | chain / verify | result SHA-256 |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for (version, evidence) in bundles {
        let report = &evidence["report"];
        let checks = &report["checks"];
        println!(
            "| {version} | {} | {}/{} | {}/{} | {} | matched | {} |",
            report["verdict"].as_str().unwrap(),
            checks["candidate_attacks_working"],
            checks["attacks_total"],
            checks["regressions_passed"],
            checks["regressions_total"],
            report["reason_codes"],
            evidence["report_sha256"].as_str().unwrap()
        );
    }
    Ok(())
}

#[tokio::test]
#[ignore = "run through scripts/demo.sh --guard with rebuilt wasm and preinstalled sandbox"]
async fn local_guard_demo() -> Result<(), Error> {
    use near_api::{Account, Signer, signer};
    use near_api_types::{AccessKeyPermission, transaction::actions::FunctionCallPermission};
    use shade_contract_template::{GuardChange, GuardState};

    let binary = PathBuf::from(std::env::var("NEAR_SANDBOX_BIN_PATH")?);
    if !binary.is_file() || !PathBuf::from(CONTRACT_WASM_PATH).is_file() {
        return Err("Missing preinstalled sandbox binary or rebuilt contract wasm".into());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()?;
    let out_dir = root.join("demo-artifacts/guard");
    std::fs::create_dir_all(&out_dir)?;
    let mut bundles = BTreeMap::new();
    for version in ["before", "after_saturate", "after_honest"] {
        let output = judge_command(&[
            "bundle",
            "--lean",
            "--subject",
            "spend",
            root.join("lean").to_str().unwrap(),
            version,
        ])?;
        let evidence: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            evidence["report"]["verdict"],
            if version == "after_honest" {
                "accept"
            } else {
                "reject"
            }
        );
        bundles.insert(version, evidence);
    }
    println!("Guard: simulated environment, synthetic data; spend policies exported and judged");
    let sandbox = Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_config: Some(json!({"telemetry":{"endpoints":[]}, "network":{"boot_nodes":"", "skip_sync_wait":true}})),
        ..Default::default()
    }).await?;
    let mut network = create_network_config(&sandbox);
    network.faucet_url = None;
    network.meta_transaction_relayer_url = None;
    network.fastnear_url = None;
    let (owner_id, owner_signer) = setup_genesis_account().await;
    let contract_id = deploy_contract_default(&network, &owner_id, &owner_signer).await?;
    call_transaction(
        &contract_id,
        "update_attestation_expiration_time",
        json!({"attestation_expiration_time_ms":"86400000"}),
        &owner_id,
        &owner_signer,
        &network,
        None,
    )
    .await?
    .assert_success();
    let (agent_id, agent_signer) =
        create_user_account(&network, &owner_id, &owner_signer, "guard-agent").await?;
    let (recipient_id, _) =
        create_user_account(&network, &owner_id, &owner_signer, "guard-recipient").await?;
    call_transaction(
        &contract_id,
        "whitelist_agent_for_local",
        json!({"account_id":agent_id}),
        &owner_id,
        &owner_signer,
        &network,
        None,
    )
    .await?
    .assert_success();
    call_transaction(
        &contract_id,
        "register_agent",
        json!({"attestation":create_mock_dstack_attestation()}),
        &agent_id,
        &agent_signer,
        &network,
        Some(DEPOSIT_005_NEAR),
    )
    .await?
    .assert_success();
    // Ephemeral key remains in memory; only its receiver/method permission is logged.
    let key = signer::generate_secret_key()?;
    Account(agent_id.clone())
        .add_key(
            AccessKeyPermission::FunctionCall(FunctionCallPermission {
                allowance: Some(near_api::NearToken::from_near(1)),
                receiver_id: contract_id.to_string(),
                method_names: vec!["guard_spend".into()],
            }),
            key.public_key(),
        )
        .with_signer(agent_signer.clone())
        .send_to(&network)
        .await?
        .assert_success();
    let spend_signer = Signer::from_secret_key(key)?;
    println!("Guard: registered agent key restricted to guard_spend on this contract");
    let baseline = &bundles["before"]["report"];
    call_transaction(&contract_id, "guard_init", json!({"limit":"1000", "spec_sha256":baseline["proof"]["approved_spec_sha256"], "policy_expr":baseline["policy_expr"]}), &owner_id, &owner_signer, &network, None).await?.assert_success();
    let recipient_before = Account(recipient_id.clone())
        .view()
        .fetch_from(&network)
        .await?
        .data
        .amount
        .as_yoctonear();
    let attack = (u64::MAX - 499).to_string();
    for amount in ["600", attack.as_str()] {
        call_transaction(
            &contract_id,
            "guard_spend",
            json!({"to":recipient_id,"amount":amount}),
            &agent_id,
            &spend_signer,
            &network,
            None,
        )
        .await?
        .assert_success();
    }
    let before = call_view::<Option<GuardState>>(&contract_id, "guard_state", json!({}), &network)
        .await?
        .data
        .unwrap();
    let recipient_after = Account(recipient_id.clone())
        .view()
        .fetch_from(&network)
        .await?
        .data
        .amount
        .as_yoctonear();
    assert_eq!(before.total_paid.0, (1u128 << 64) + 100);
    assert_eq!(recipient_after - recipient_before, before.total_paid.0);
    assert_eq!(before.spent.0, 100);
    assert_eq!(before.pending_paid.0, 0);
    println!(
        "Guard: overflow attack transferred {} yoctoNEAR; limit=1000, wrapped spent=100",
        before.total_paid.0
    );
    let saturate = &bundles["after_saturate"]["report"]["policy_expr"];
    let missing = call_transaction(
        &contract_id,
        "guard_set_policy",
        json!({"policy_expr":saturate,"record_id":"missing"}),
        &owner_id,
        &owner_signer,
        &network,
        None,
    )
    .await?;
    assert!(missing.is_failure());
    assert!(format!("{missing:?}").contains("Guard record missing"));
    println!("Guard: replacement without a record rejected (Guard record missing)");
    for version in ["after_saturate", "after_honest"] {
        let evidence = bundles.get_mut(version).unwrap();
        let id = evidence["report_sha256"].as_str().unwrap().to_owned();
        call_transaction(
            &contract_id,
            "submit_fix_proof",
            json!({"bundle_hash":id,"summary":proof_summary(&evidence["report"])?}),
            &agent_id,
            &agent_signer,
            &network,
            None,
        )
        .await?
        .assert_success();
        let record = call_view::<Option<FixProof>>(
            &contract_id,
            "get_fix_proof",
            json!({"bundle_hash":id}),
            &network,
        )
        .await?
        .data
        .unwrap();
        evidence["chain"] = serde_json::to_value(record)?;
        let path = out_dir.join(format!("{version}.json"));
        std::fs::write(&path, serde_json::to_vec(evidence)?)?;
        judge_command(&["verify", path.to_str().unwrap()])?;
        let update = call_transaction(
            &contract_id,
            "guard_set_policy",
            json!({"policy_expr":evidence["report"]["policy_expr"], "record_id":id}),
            &owner_id,
            &owner_signer,
            &network,
            None,
        )
        .await?;
        if version == "after_saturate" {
            assert!(update.is_failure());
            assert!(format!("{update:?}").contains("Guard record is not accept"));
            println!("Guard: rejected saturation record could not replace policy");
        } else {
            update.assert_success();
            println!(
                "Guard: accept record installed the exported policy; chain readback and replay matched"
            );
        }
    }
    let failed = call_transaction(
        &contract_id,
        "guard_spend",
        json!({"to":recipient_id,"amount":attack}),
        &agent_id,
        &spend_signer,
        &network,
        None,
    )
    .await?;
    assert!(failed.is_failure());
    assert!(format!("{failed:?}").contains("Guard spending denied"));
    let unchanged = Account(recipient_id.clone())
        .view()
        .fetch_from(&network)
        .await?
        .data
        .amount
        .as_yoctonear();
    assert_eq!(unchanged, recipient_after);
    println!("Guard: same overflow attack rejected; recipient balance unchanged");
    call_transaction(
        &contract_id,
        "guard_spend",
        json!({"to":recipient_id,"amount":"100"}),
        &agent_id,
        &spend_signer,
        &network,
        None,
    )
    .await?
    .assert_success();
    let after = call_view::<Option<GuardState>>(&contract_id, "guard_state", json!({}), &network)
        .await?
        .data
        .unwrap();
    assert_eq!(after.total_paid.0, before.total_paid.0 + 100);
    assert_eq!(after.spent.0, 200);
    assert_eq!(
        Account(recipient_id)
            .view()
            .fetch_from(&network)
            .await?
            .data
            .amount
            .as_yoctonear(),
        recipient_after + 100
    );
    let history = call_view::<Vec<GuardChange>>(&contract_id, "guard_history", json!({}), &network)
        .await?
        .data;
    assert_eq!(history.len(), 1);
    let restricted = call_transaction(
        &contract_id,
        "guard_set_policy",
        json!({"policy_expr":after.policy_expr,"record_id":after.record_id}),
        &agent_id,
        &spend_signer,
        &network,
        None,
    )
    .await;
    assert!(
        restricted.is_err(),
        "access key must reject other methods before execution"
    );
    println!(
        "Guard: legitimate 100 yoctoNEAR paid; history=1; restricted key rejected guard_set_policy"
    );
    Ok(())
}
