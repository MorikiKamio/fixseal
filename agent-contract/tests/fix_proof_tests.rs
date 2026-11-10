use near_sdk::json_types::U64;
use near_sdk::test_utils::{VMContextBuilder, accounts};
use near_sdk::{AccountId, NearToken, PromiseOrValue, env, testing_env};
use shade_attestation::{
    attestation::create_mock_dstack_attestation, measurements::create_mock_full_measurements_hex,
};
use shade_contract_template::{Contract, Ppid};
use std::panic::{AssertUnwindSafe, catch_unwind};

// Local mock execution only; this does not exercise a real TEE or sandbox receipts.
fn set_context(predecessor: AccountId, timestamp_ms: u64, deposit: NearToken) {
    let mut context = VMContextBuilder::new();
    context
        .current_account_id(accounts(0))
        .signer_account_id(accounts(1))
        .predecessor_account_id(predecessor)
        .block_timestamp(timestamp_ms * 1_000_000)
        .attached_deposit(deposit);
    testing_env!(context.build());
}

fn setup_contract() -> Contract {
    set_context(accounts(0), 1000, NearToken::from_near(0));
    let mut contract = Contract::new(false, U64(100_000), accounts(0), accounts(1));
    contract.approve_measurements(create_mock_full_measurements_hex());
    contract.approve_ppids(vec![Ppid::default()]);
    contract
}

fn register_agent(contract: &mut Contract, account_id: AccountId) {
    set_context(accounts(0), 1000, NearToken::from_near(0));
    contract.whitelist_agent_for_local(account_id.clone());
    set_context(
        account_id.clone(),
        1000,
        NearToken::from_yoctonear(5_000_000_000_000_000_000_000),
    );
    assert!(contract.register_agent(create_mock_dstack_attestation()));
    set_context(account_id, 2000, NearToken::from_near(0));
}

#[test]
fn test_registered_agent_can_submit_fix_proof() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));
    let bundle_hash = "ab".repeat(32);

    let result = contract.submit_fix_proof(bundle_hash.clone(), "Dummy replay result".to_string());

    assert!(matches!(result, PromiseOrValue::Value(())));
    let proof = contract.get_fix_proof(bundle_hash.clone()).unwrap();
    assert_eq!(proof.bundle_hash, bundle_hash);
    assert_eq!(proof.caller, accounts(2));
    assert_ne!(proof.caller, env::signer_account_id());
    assert_eq!(proof.block_timestamp_ms, U64(2000));
    assert_eq!(proof.summary, "Dummy replay result");
    assert_eq!(
        serde_json::to_value(&proof).unwrap()["block_timestamp_ms"],
        "2000"
    );
}

#[test]
#[should_panic(expected = "Agent not registered")]
fn test_unregistered_agent_cannot_submit_fix_proof() {
    let mut contract = setup_contract();
    set_context(accounts(2), 2000, NearToken::from_near(0));

    let _ = contract.submit_fix_proof("ab".repeat(32), "Dummy result".to_string());
}

#[test]
#[should_panic(expected = "Agent not registered")]
fn test_whitelist_alone_does_not_allow_submission() {
    let mut contract = setup_contract();
    contract.whitelist_agent_for_local(accounts(2));
    set_context(accounts(2), 2000, NearToken::from_near(0));

    let _ = contract.submit_fix_proof("ab".repeat(32), "Dummy result".to_string());
}

#[test]
fn test_duplicate_fix_proof_cannot_be_overwritten_by_another_agent() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));
    let bundle_hash = "ab".repeat(32);
    let _ = contract.submit_fix_proof(bundle_hash.clone(), "Original dummy result".to_string());
    let original = contract.get_fix_proof(bundle_hash.clone()).unwrap();

    register_agent(&mut contract, accounts(3));
    let result = catch_unwind(AssertUnwindSafe(|| {
        contract.submit_fix_proof(bundle_hash.clone(), "Replacement dummy result".to_string())
    }));

    assert!(result.is_err());
    assert_eq!(contract.get_fix_proof(bundle_hash), Some(original));
    assert_eq!(contract.list_fix_proofs(&None, &None).len(), 1);
}

#[test]
fn test_fix_proof_views_and_pagination() {
    let mut contract = setup_contract();
    assert!(contract.get_fix_proof("missing".to_string()).is_none());
    assert!(contract.list_fix_proofs(&None, &None).is_empty());
    register_agent(&mut contract, accounts(2));

    // Reverse hash order to distinguish insertion order from key order.
    for hash in ["cd".repeat(32), "ab".repeat(32), "ef".repeat(32)] {
        let _ = contract.submit_fix_proof(hash, "Dummy result".to_string());
    }
    let all = contract.list_fix_proofs(&None, &None);
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].bundle_hash, "cd".repeat(32));
    assert_eq!(all[1].bundle_hash, "ab".repeat(32));
    assert_eq!(all[2].bundle_hash, "ef".repeat(32));
    assert_eq!(contract.list_fix_proofs(&Some(1), &Some(1)), all[1..2]);
    assert_eq!(contract.list_fix_proofs(&Some(1), &None), all[1..]);
    assert_eq!(contract.list_fix_proofs(&None, &Some(1)), all[..1]);
    assert!(contract.list_fix_proofs(&Some(3), &Some(1)).is_empty());
    assert!(contract.list_fix_proofs(&Some(u32::MAX), &None).is_empty());
    assert!(contract.list_fix_proofs(&None, &Some(0)).is_empty());
}

#[test]
fn test_fix_proof_metadata_stays_constant_as_records_grow() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));
    let mut metadata_size = None;

    for count in 1..=100 {
        set_context(accounts(2), 2000, NearToken::from_near(0));
        let result = contract.submit_fix_proof(format!("{count:064x}"), "x".repeat(100));
        assert!(matches!(result, PromiseOrValue::Value(())));
        let size = env::storage_read(b"FIX_PROOFS").unwrap().len();
        assert_eq!(size, *metadata_size.get_or_insert(size));
        if [1, 10, 100].contains(&count) {
            println!("records={count} FIX_PROOFS_bytes={size}");
        }
    }

    set_context(accounts(2), 2000, NearToken::from_near(0));
    for count in [1, 10, 100] {
        let hash = format!("{count:064x}");
        let proof = contract.get_fix_proof(hash.clone()).unwrap();
        assert_eq!(proof.bundle_hash, hash);
        assert_eq!(proof.summary, "x".repeat(100));
    }
    let proofs = contract.list_fix_proofs(&None, &None);
    assert_eq!(proofs.len(), 100);
    assert_eq!(proofs[0].bundle_hash, format!("{:064x}", 1));
    assert_eq!(proofs[99].bundle_hash, format!("{:064x}", 100));
}

#[test]
fn test_fix_proof_survives_state_reload() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));
    let bundle_hash = "ab".repeat(32);
    let _ = contract.submit_fix_proof(bundle_hash.clone(), "Dummy result".to_string());
    let original = contract.get_fix_proof(bundle_hash.clone()).unwrap();
    env::state_write(&contract);
    drop(contract);

    let contract: Contract = env::state_read().unwrap();
    assert_eq!(contract.get_fix_proof(bundle_hash), Some(original.clone()));
    assert_eq!(contract.list_fix_proofs(&None, &None), vec![original]);
}

fn assert_invalid_agent_cannot_submit(mut contract: Contract, timestamp_ms: u64) {
    set_context(accounts(2), timestamp_ms, NearToken::from_near(0));
    let bundle_hash = "ab".repeat(32);
    let result = contract.submit_fix_proof(bundle_hash.clone(), "Dummy result".to_string());

    assert!(matches!(result, PromiseOrValue::Promise(_)));
    assert!(contract.get_agent(accounts(2)).is_none());
    assert!(contract.get_fix_proof(bundle_hash).is_none());
    assert!(contract.list_fix_proofs(&None, &None).is_empty());
}

#[test]
fn test_expired_agent_cannot_submit_fix_proof() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));

    assert_invalid_agent_cannot_submit(contract, 101_001);
}

#[test]
fn test_agent_with_revoked_measurements_cannot_submit_fix_proof() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));
    set_context(accounts(0), 2000, NearToken::from_near(0));
    contract.remove_measurements(create_mock_full_measurements_hex());

    assert_invalid_agent_cannot_submit(contract, 2000);
}

#[test]
fn test_agent_with_revoked_ppid_cannot_submit_fix_proof() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));
    set_context(accounts(0), 2000, NearToken::from_near(0));
    contract.remove_ppids(vec![Ppid::default()]);

    assert_invalid_agent_cannot_submit(contract, 2000);
}

#[test]
fn test_agent_removed_from_local_whitelist_cannot_submit_fix_proof() {
    let mut contract = setup_contract();
    register_agent(&mut contract, accounts(2));
    set_context(accounts(0), 2000, NearToken::from_near(0));
    contract.remove_agent_from_whitelist_for_local(accounts(2));

    assert_invalid_agent_cannot_submit(contract, 2000);
}
