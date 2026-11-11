# Contract behavior: English summary

`submit_fix_proof` records an authorized registered agent's result hash and summary; the contract does not run Lean or approve the verdict. `get_fix_proof` reads the stored claim. A matching offline readback hash does not authenticate its provenance.

Registration uses the predecessor account, requires storage deposit for a new record, and stores measurement, PPID and expiry. Local mode uses mock attestation and requires owner whitelist plus approved measurement/PPID; whitelisting alone is not registration. Submission checks registration, expiry, measurement, PPID and local whitelist before saving. Invalid agents are removed and return a failing promise; views expose validity without removing records. Duplicate result hashes cannot be overwritten by another agent. There is no general storage quota or fee recovery mechanism for recorded claims; this is a demo API limitation.

Use cached dependencies and `cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests` for the mock authorization tests. Sandbox integration needs supplied wasm, near-sandbox 2.11.0 and explicit localhost bind permission. The operator's [FIX5 record](demo-output-host-fix5.txt) covers ten versions on path-7/spend-3 with matched readbacks/replays and demo exit 0. All measurements describe a **simulated environment, synthetic data**. Real TEE, testnet and supplied wasm/source correspondence remain unconfirmed. Details and historical API notes follow.

## Detailed reference

# T001 contract notes

Updated: 2026-11-10 (R2)

## Template registration and authorization

- `src/lib.rs::register_agent` registers `env::predecessor_account_id()`. New registration requires deposit ≥ `storage_byte_cost × 486 bytes`; renewal updates the existing record without that deposit check.
- `src/internal/attestation.rs::verify_attestation` branches on `requires_tee`. TEE mode requires a lowercase hexadecimal implicit account and checks attestation against account-bound report data, current time and owner-approved measurement / PPID.
- Local mode requires owner `whitelist_agent_for_local` and approved default measurement / PPID. Attestation is dummy; local mode is not a real TEE. Whitelisting still requires `register_agent` afterward.
- Registration stores measurement, PPID and `valid_until_ms = current block time (ms) + validity period` in `agents`, then emits `AgentRegistered`.
- `src/internal/helpers.rs::require_valid_agent` panics with `Agent not registered` if absent. Existing registration becomes invalid after expiry, measurement / PPID revocation or local whitelist removal. Equality of expiry and current time remains valid.
- Invalid registration is removed, `AgentRemoved` emitted, and a Promise calls private `fail_on_invalid_agent`. Returning that Promise preserves removal while failing a later receipt. `get_agent` / `get_agents` show the same validity reasons without deleting registration.
- Approval, revocation and whitelist operations use `require_owner` to compare predecessor and owner. Registration alone does not establish current validity.

## Added Fixseal API

Added in `src/your_functions.rs`; `request_signature` and existing authorization are unchanged.

| Method | Arguments | Behavior |
| --- | --- | --- |
| `submit_fix_proof` | `bundle_hash: String`, `summary: String` | Stores only for agents passing `require_valid_agent`; duplicate hash strings reject across all agents. Success returns JSON `null`; agents invalidated after registration receive the existing failure Promise. |
| `get_fix_proof` | `bundle_hash: String` | Public view returning the record or `null` when absent. |
| `list_fix_proofs` | `from_index: Option<u32>`, `limit: Option<u32>` | Public view in insertion order. Omitted / `null` reads all from the beginning; out-of-range or zero limit returns an empty array. |

Records contain `bundle_hash`, `caller`, `block_timestamp_ms`, `summary`. Caller is immediate `predecessor_account_id`, not `signer_account_id`. Contract-observed milliseconds use `U64`, serialized as a JSON string. Records remain visible after agent invalidation.

Local writes require registration and validity checks, not just whitelisting. The existing `src/index.ts` startup loop whitelists then registers, matching this flow.

Records use `IterableMap<String, FixProof>`: map metadata under `FIX_PROOFS`, elements under appended enum variant `StorageKey::FixProofs`, flushed on writes. Existing `Contract` Borsh layout and storage-key numbers are preserved so template upgrade-test WASM can read old state. Upgrade execution was unconfirmed.

SDK persistent collections load elements lazily and `flush` writes changed elements, without rewriting all records under `FIX_PROOFS`. R2 **simulated environment, synthetic data** checks found 16-byte metadata at 1, 10 and 100 records, with first/last reads and insertion order intact. This is not real-chain gas measurement.

This non-payable demo API charges record storage to contract funds. String length, record count and registered-agent capacity use are uncapped; no cost recovery exists. T001 treats hashes as raw strings without format/normalization, bundle, signature or attack-result checks; those belong to later pipeline / verify work.

## Validation status

`tests/fix_proof_tests.rs` contains these 11 tests using the NEAR SDK mocked VM: **simulated environment, synthetic data**. All 11 passed in R2; they do not establish real TEE, NEAR testnet or sandbox receipt execution.

- Registered-agent success, caller, time, summary and JSON time.
- Unregistered-agent rejection.
- Rejection of local-whitelisted but unregistered callers.
- Duplicate hash rejection across registered agents while preserving the original record.
- Missing reads, insertion order, pagination, out-of-range and zero limit.
- Reads after state serialization/reload.
- Constant metadata size and intact reads/order through 100 records.
- Expired-agent rejection and registration removal.
- Measurement revocation rejection and removal.
- PPID revocation rejection and removal.
- Local whitelist revocation rejection and removal.

R1 used existing caches: 51 contract unit, 10 fix_proof and 1 summary-format tests passed. All 9 sandbox contract tests failed at localhost bind with EPERM. The operator's pre-R1 host demo recorded/read/replayed all 5 versions ([record](demo-output-host.txt)). New tests also do not execute the final failing receipt of an invalid-agent Promise. See [BLOCKED.md](BLOCKED.md) for resumption steps.
