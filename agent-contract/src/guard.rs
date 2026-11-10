use crate::*;
use near_sdk::json_types::U128;
use policy_expr::BoolExpr;

const GUARD_STATE_KEY: &[u8] = b"SPEND_GUARD";
const GUARD_HISTORY_KEY: &[u8] = b"SPEND_GUARD_HISTORY";
const ALLOWED_JUDGES: &[&str] = &["0.1.0/lean-spend-4"];

#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardState {
    pub limit: U64,
    pub spent: U64,
    pub total_paid: U128,
    pub pending_paid: U128,
    pub spec_sha256: String,
    pub policy_expr: String,
    pub policy_expr_sha256: String,
    pub record_id: Option<String>,
}

#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardChange {
    pub record_id: String,
    pub previous_policy_expr_sha256: String,
    pub policy_expr_sha256: String,
    pub block_timestamp_ms: U64,
}

#[derive(serde::Deserialize)]
struct GuardRecord {
    verdict: String,
    subject: String,
    spec_sha256: String,
    policy_expr_sha256: String,
    policy_expr: String,
    judge_version: String,
}

fn parse_policy(json: &str) -> BoolExpr {
    BoolExpr::parse(json).unwrap_or_else(|error| env::panic_str(error))
}

fn policy_hash(json: &str) -> String {
    hex::encode(env::sha256(json.as_bytes()))
}

fn save_state(state: &GuardState) {
    env::storage_write(
        GUARD_STATE_KEY,
        &serde_json::to_vec(state).expect("Failed to serialize guard state"),
    );
}

fn event(name: &str, data: serde_json::Value) {
    log!(
        "EVENT_JSON:{}",
        serde_json::json!({
            "standard": "fixseal_guard", "version": "1.0.0", "event": name, "data": [data]
        })
    );
}

#[near]
impl Contract {
    pub fn guard_init(&mut self, limit: U64, spec_sha256: String, policy_expr: String) {
        self.require_owner();
        require!(self.guard_state().is_none(), "Guard already initialized");
        require!(
            spec_sha256.len() == 64
                && spec_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Invalid guard spec hash"
        );
        let expr = parse_policy(&policy_expr);
        require!(
            expr == BoolExpr::vulnerable(),
            "Guard initial policy must be vulnerable baseline"
        );
        let policy_expr = expr.canonical();
        let state = GuardState {
            limit,
            spent: U64(0),
            total_paid: U128(0),
            pending_paid: U128(0),
            spec_sha256,
            policy_expr_sha256: policy_hash(&policy_expr),
            policy_expr,
            record_id: None,
        };
        save_state(&state);
        event("init", serde_json::to_value(state).unwrap());
    }

    // Install the restricted key on a registered agent account, with this receiver and method only.
    pub fn guard_spend(&mut self, to: AccountId, amount: U64) -> Promise {
        let caller = env::predecessor_account_id();
        let agent = self
            .agents
            .get(&caller)
            .expect("Guard agent not registered");
        require!(
            self.check_invalid_reasons(&caller, agent).is_empty(),
            "Guard agent invalid"
        );
        let mut state = self.guard_state().expect("Guard not initialized");
        require!(
            parse_policy(&state.policy_expr).eval(state.limit.0, state.spent.0, amount.0),
            "Guard spending denied"
        );
        let pending = state
            .pending_paid
            .0
            .checked_add(u128::from(amount.0))
            .expect("Guard pending overflow");
        state
            .total_paid
            .0
            .checked_add(pending)
            .expect("Guard total overflow");
        state.pending_paid = U128(pending);
        state.spent = U64(state.spent.0.wrapping_add(amount.0));
        save_state(&state);
        event(
            "spend_requested",
            serde_json::json!({"caller": caller, "to": to, "amount": amount, "spent": state.spent}),
        );
        Promise::new(to)
            .transfer(NearToken::from_yoctonear(u128::from(amount.0)))
            .then(Promise::new(env::current_account_id()).function_call(
                "guard_on_transfer",
                serde_json::to_vec(&serde_json::json!({"amount": amount})).unwrap(),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(12),
            ))
    }

    #[private]
    pub fn guard_on_transfer(&mut self, amount: U64) -> bool {
        require!(
            env::predecessor_account_id() == env::current_account_id(),
            "Guard callback is private"
        );
        require!(
            env::promise_results_count() == 1,
            "Guard callback requires one result"
        );
        let mut state = self.guard_state().expect("Guard not initialized");
        state.pending_paid = U128(
            state
                .pending_paid
                .0
                .checked_sub(u128::from(amount.0))
                .expect("Guard pending underflow"),
        );
        let paid = env::promise_result_checked(0, 0).is_ok();
        if paid {
            state.total_paid = U128(
                state
                    .total_paid
                    .0
                    .checked_add(u128::from(amount.0))
                    .expect("Guard total overflow"),
            );
        } else {
            state.spent = U64(state.spent.0.wrapping_sub(amount.0));
        }
        save_state(&state);
        event(
            "spend_settled",
            serde_json::json!({"amount": amount, "paid": paid, "total_paid": state.total_paid}),
        );
        paid
    }

    pub fn guard_set_policy(&mut self, policy_expr: String, record_id: String) {
        self.require_owner();
        let mut state = self.guard_state().expect("Guard not initialized");
        let record = self
            .get_fix_proof(record_id.clone())
            .expect("Guard record missing");
        let record: GuardRecord =
            serde_json::from_str(&record.summary).expect("Guard record malformed");
        require!(record.verdict == "accept", "Guard record is not accept");
        require!(
            record.subject == "spend",
            "Guard record subject is not spend"
        );
        require!(
            record.spec_sha256 == state.spec_sha256,
            "Guard record spec mismatch"
        );
        let policy_expr = parse_policy(&policy_expr).canonical();
        let hash = policy_hash(&policy_expr);
        require!(
            record.policy_expr_sha256 == hash,
            "Guard record policy hash mismatch"
        );
        require!(
            parse_policy(&record.policy_expr).canonical() == policy_expr,
            "Guard record expression mismatch"
        );
        require!(
            ALLOWED_JUDGES.contains(&record.judge_version.as_str()),
            "Guard record judge not allowed"
        );
        let change = GuardChange {
            record_id: record_id.clone(),
            previous_policy_expr_sha256: state.policy_expr_sha256.clone(),
            policy_expr_sha256: hash.clone(),
            block_timestamp_ms: U64(block_timestamp_ms()),
        };
        let mut history = self.guard_history();
        history.push(change.clone());
        env::storage_write(GUARD_HISTORY_KEY, &serde_json::to_vec(&history).unwrap());
        state.policy_expr = policy_expr;
        state.policy_expr_sha256 = hash;
        state.record_id = Some(record_id);
        save_state(&state);
        event("policy_changed", serde_json::to_value(change).unwrap());
    }

    pub fn guard_state(&self) -> Option<GuardState> {
        env::storage_read(GUARD_STATE_KEY)
            .map(|bytes| serde_json::from_slice(&bytes).expect("Failed to read guard state"))
    }

    pub fn guard_history(&self) -> Vec<GuardChange> {
        env::storage_read(GUARD_HISTORY_KEY)
            .map(|bytes| serde_json::from_slice(&bytes).expect("Failed to read guard history"))
            .unwrap_or_default()
    }
}
