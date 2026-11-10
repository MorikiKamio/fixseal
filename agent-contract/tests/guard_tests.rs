use near_sdk::json_types::{U64, U128};
use near_sdk::test_utils::{VMContextBuilder, accounts, get_created_receipts, get_logs};
use near_sdk::{NearToken, PromiseOrValue, PromiseResult, RuntimeFeesConfig, env, testing_env};
use shade_attestation::{
    attestation::create_mock_dstack_attestation, measurements::create_mock_full_measurements_hex,
};
use shade_contract_template::{
    Contract, Ppid,
    policy_expr::{BoolExpr as B, UIntExpr as U},
};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

const SPEC: &str = "e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe";

fn context(who: usize) -> VMContextBuilder {
    let mut c = VMContextBuilder::new();
    c.current_account_id(accounts(0))
        .predecessor_account_id(accounts(who))
        .signer_account_id(accounts(who))
        .block_timestamp(1_000_000_000);
    testing_env!(c.build());
    c
}

fn setup() -> Contract {
    context(0);
    let mut contract = Contract::new(false, U64(100_000), accounts(0), accounts(1));
    contract.approve_measurements(create_mock_full_measurements_hex());
    contract.approve_ppids(vec![Ppid::default()]);
    contract.whitelist_agent_for_local(accounts(2));
    let mut c = context(2);
    c.attached_deposit(NearToken::from_near(1));
    testing_env!(c.build());
    contract.register_agent(create_mock_dstack_attestation());
    context(0);
    contract.guard_init(U64(1000), SPEC.into(), B::vulnerable().canonical());
    contract
}

fn corrected() -> String {
    B::And(
        Box::new(B::Le(U::Amount, U::Limit)),
        Box::new(B::Le(
            U::Spent,
            U::Sub(Box::new(U::Limit), Box::new(U::Amount)),
        )),
    )
    .canonical()
}

fn hash(json: &str) -> String {
    hex::encode(env::sha256(json.as_bytes()))
}

fn record(expr: &str) -> serde_json::Value {
    serde_json::json!({
        "verdict":"accept", "subject":"spend", "spec_sha256":SPEC,
        "policy_expr": expr, "policy_expr_sha256":hash(expr),
        "judge_version":"0.1.0/lean-spend-4", "inputs_sha256":"ab".repeat(32)
    })
}

fn submit(contract: &mut Contract, id: &str, summary: String) {
    context(2);
    assert!(matches!(
        contract.submit_fix_proof(id.into(), summary),
        PromiseOrValue::Value(())
    ));
    context(0);
}

fn panic_text(f: impl FnOnce()) -> String {
    let panic = catch_unwind(AssertUnwindSafe(f)).expect_err("expected rejection");
    if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = panic.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "unknown panic".into()
    }
}

fn settle(contract: &mut Contract, amount: u64, paid: bool) {
    let c = context(0);
    testing_env!(
        c.build(),
        near_sdk::test_vm_config(),
        RuntimeFeesConfig::test(),
        HashMap::default(),
        vec![if paid {
            PromiseResult::Successful(vec![])
        } else {
            PromiseResult::Failed
        }]
    );
    assert_eq!(contract.guard_on_transfer(U64(amount)), paid);
}

#[test]
fn every_record_rejection_preserves_policy_and_history() {
    let mut contract = setup();
    let expr = corrected();
    let original = contract.guard_state();
    assert!(
        panic_text(|| contract.guard_set_policy(expr.clone(), "missing".into()))
            .contains("Guard record missing")
    );
    submit(&mut contract, "malformed", "legacy summary".into());
    assert!(
        panic_text(|| contract.guard_set_policy(expr.clone(), "malformed".into()))
            .contains("Guard record malformed")
    );
    for (field, value, error) in [
        ("verdict", "reject", "Guard record is not accept"),
        ("subject", "path", "Guard record subject is not spend"),
        ("spec_sha256", "wrong", "Guard record spec mismatch"),
        (
            "policy_expr_sha256",
            "wrong",
            "Guard record policy hash mismatch",
        ),
        (
            "policy_expr",
            r#"{"op":"true"}"#,
            "Guard record expression mismatch",
        ),
        (
            "judge_version",
            "0.1.0/lean-spend-3",
            "Guard record judge not allowed",
        ),
    ] {
        let mut summary = record(&expr);
        summary[field] = value.into();
        submit(&mut contract, field, summary.to_string());
        let reason = panic_text(|| contract.guard_set_policy(expr.clone(), field.into()));
        assert!(reason.contains(error), "{reason}");
        assert_eq!(contract.guard_state(), original);
        assert!(contract.guard_history().is_empty());
    }
}

#[test]
fn overflow_transfer_then_record_bound_replacement() {
    let mut contract = setup();
    context(2);
    drop(contract.guard_spend(accounts(3), U64(600)));
    assert!(
        get_created_receipts()
            .iter()
            .any(|r| r.receiver_id == accounts(3))
    );
    assert_eq!(contract.guard_state().unwrap().pending_paid, U128(600));
    assert_eq!(contract.guard_state().unwrap().total_paid, U128(0));
    settle(&mut contract, 600, true);
    let attack = u64::MAX - 499;
    context(2);
    drop(contract.guard_spend(accounts(3), U64(attack)));
    settle(&mut contract, attack, true);
    let before = contract.guard_state().unwrap();
    assert_eq!(before.spent, U64(100));
    assert_eq!(before.total_paid, U128((1u128 << 64) + 100));
    let expr = corrected();
    submit(&mut contract, "accepted", record(&expr).to_string());
    // Reordered keys and whitespace hash to the same canonical expression.
    let reformatted: serde_json::Value = serde_json::from_str(&expr).unwrap();
    contract.guard_set_policy(
        serde_json::to_string_pretty(&reformatted).unwrap(),
        "accepted".into(),
    );
    assert_eq!(contract.guard_history().len(), 1);
    assert_eq!(
        contract.guard_state().unwrap().record_id.as_deref(),
        Some("accepted")
    );
    assert!(get_logs().iter().any(|l| l.contains("policy_changed")));
    context(2);
    assert!(
        panic_text(|| {
            drop(contract.guard_spend(accounts(3), U64(attack)));
        })
        .contains("Guard spending denied")
    );
    assert_eq!(
        contract.guard_state().unwrap().total_paid,
        before.total_paid
    );
    context(2);
    drop(contract.guard_spend(accounts(3), U64(100)));
    settle(&mut contract, 100, true);
    let after = contract.guard_state().unwrap();
    assert_eq!(after.spent, U64(200));
    assert_eq!(after.total_paid.0, before.total_paid.0 + 100);
    env::state_write(&contract);
    drop(contract);
    let contract: Contract = env::state_read().unwrap();
    assert_eq!(contract.guard_state().unwrap(), after);
    assert_eq!(contract.guard_history().len(), 1);
}

#[test]
fn failed_transfer_releases_only_its_reservation() {
    let mut contract = setup();
    context(2);
    drop(contract.guard_spend(accounts(3), U64(600)));
    context(2);
    drop(contract.guard_spend(accounts(3), U64(200)));
    settle(&mut contract, 200, true);
    settle(&mut contract, 600, false);
    let state = contract.guard_state().unwrap();
    assert_eq!(state.spent, U64(200));
    assert_eq!(state.pending_paid, U128(0));
    assert_eq!(state.total_paid, U128(200));
}

#[test]
fn authorization_initialization_and_callback_boundaries() {
    let mut contract = setup();
    let original = contract.guard_state();
    context(2);
    assert!(
        panic_text(|| contract.guard_set_policy(corrected(), "any".into()))
            .contains("Caller is not the owner")
    );
    assert!(
        panic_text(|| contract.guard_init(U64(1), SPEC.into(), B::vulnerable().canonical()))
            .contains("Caller is not the owner")
    );
    assert!(
        panic_text(|| {
            contract.guard_on_transfer(U64(0));
        })
        .contains("Guard callback is private")
    );
    context(0);
    assert!(
        panic_text(|| contract.guard_init(U64(1000), SPEC.into(), B::vulnerable().canonical()))
            .contains("Guard already initialized")
    );
    context(3);
    assert!(
        panic_text(|| {
            drop(contract.guard_spend(accounts(3), U64(1)));
        })
        .contains("Guard agent not registered")
    );
    context(0);
    contract.remove_agent_from_whitelist_for_local(accounts(2));
    context(2);
    assert!(
        panic_text(|| {
            drop(contract.guard_spend(accounts(3), U64(1)));
        })
        .contains("Guard agent invalid")
    );
    assert_eq!(contract.guard_state(), original);
}

#[test]
fn canonical_hash_known_vector() {
    context(0);
    assert_eq!(
        hash(r#"{"op":"true"}"#),
        "cddde364f5f8a94897d2de0ce0d91a1e51d1208908ed04cac3786fe4dd80fa70"
    );
}

#[test]
fn initialization_rejects_unreviewable_hash_and_nonbaseline_expression() {
    context(0);
    let mut contract = Contract::new(false, U64(100_000), accounts(0), accounts(1));
    assert!(contract.guard_state().is_none());
    assert!(
        panic_text(|| contract.guard_init(U64(1000), "wrong".into(), B::vulnerable().canonical()))
            .contains("Invalid guard spec hash")
    );
    assert!(
        panic_text(|| contract.guard_init(U64(1000), SPEC.into(), corrected()))
            .contains("Guard initial policy must be vulnerable baseline")
    );
    assert!(contract.guard_state().is_none());
    assert!(contract.guard_history().is_empty());
}
