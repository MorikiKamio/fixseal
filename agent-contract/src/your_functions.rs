use crate::*;
use near_sdk::PromiseOrValue;

const FIX_PROOFS_STATE_KEY: &[u8] = b"FIX_PROOFS";

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixProof {
    pub bundle_hash: String,
    pub caller: AccountId,
    pub block_timestamp_ms: U64,
    pub summary: String,
}

// Keep proof storage separate to preserve the template's contract state layout.
fn load_fix_proofs() -> IterableMap<String, FixProof> {
    match env::storage_read(FIX_PROOFS_STATE_KEY) {
        Some(bytes) => near_sdk::borsh::from_slice(&bytes).expect("Failed to read fix proofs"),
        None => IterableMap::new(StorageKey::FixProofs),
    }
}

#[near]
impl Contract {
    // Record a fix proof if the caller is a valid registered agent
    pub fn submit_fix_proof(&mut self, bundle_hash: String, summary: String) -> PromiseOrValue<()> {
        if let Some(failure_promise) = self.require_valid_agent() {
            return PromiseOrValue::Promise(failure_promise);
        }

        let mut fix_proofs = load_fix_proofs();
        require!(
            !fix_proofs.contains_key(&bundle_hash),
            "Fix proof already registered"
        );

        fix_proofs.insert(
            bundle_hash.clone(),
            FixProof {
                bundle_hash,
                caller: env::predecessor_account_id(),
                block_timestamp_ms: U64::from(block_timestamp_ms()),
                summary,
            },
        );
        fix_proofs.flush();
        env::storage_write(
            FIX_PROOFS_STATE_KEY,
            &near_sdk::borsh::to_vec(&fix_proofs).expect("Failed to serialize fix proofs"),
        );

        PromiseOrValue::Value(())
    }

    pub fn get_fix_proof(&self, bundle_hash: String) -> Option<FixProof> {
        load_fix_proofs().get(&bundle_hash).cloned()
    }

    // List fix proofs in insertion order
    pub fn list_fix_proofs(&self, from_index: &Option<u32>, limit: &Option<u32>) -> Vec<FixProof> {
        let fix_proofs = load_fix_proofs();
        let from = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(fix_proofs.len());

        fix_proofs
            .values()
            .skip(from as usize)
            .take(limit as usize)
            .cloned()
            .collect()
    }
}

// Request a signature for a transaction payload if its a valid agent
#[near]
impl Contract {
    pub fn request_signature(
        &mut self,
        path: String,
        payload: String,
        key_type: String,
    ) -> Promise {
        // Require the caller to be a valid agent, if not, execute a promise to panic
        if let Some(failure_promise) = self.require_valid_agent() {
            return failure_promise;
        }

        self.internal_request_signature(path, payload, key_type)
    }
}
