# Contract behavior: English summary

`submit_fix_proof` records an authorized registered agent's result hash and summary; the contract does not run Lean or approve the verdict. `get_fix_proof` reads the stored claim. A matching offline readback hash does not authenticate its provenance.

Registration uses the predecessor account, requires storage deposit for a new record, and stores measurement, PPID and expiry. Local mode uses mock attestation and requires owner whitelist plus approved measurement/PPID; whitelisting alone is not registration. Submission checks registration, expiry, measurement, PPID and local whitelist before saving. Invalid agents are removed and return a failing promise; views expose validity without removing records. Duplicate result hashes cannot be overwritten by another agent. There is no general storage quota or fee recovery mechanism for recorded claims; this is a demo API limitation.

Use cached dependencies and `cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests` for the mock authorization tests. Sandbox integration needs supplied wasm, near-sandbox 2.11.0 and explicit localhost bind permission. The operator's [FIX5 record](demo-output-host-fix5.txt) covers ten versions on path-7/spend-3 with matched readbacks/replays and demo exit 0. All measurements describe a **simulated environment, synthetic data**. Real TEE, testnet and supplied wasm/source correspondence remain unconfirmed. Details and historical API notes follow.

## Japanese reference

# T001 契約メモ

更新: 2026-11-10（R2）

## テンプレートの登録と認可

- `src/lib.rs::register_agent` は `env::predecessor_account_id()` を登録先にする。新規登録は `storage_byte_cost × 486 bytes` 以上の deposit が必要。再登録は既存レコードを更新するため、この deposit 判定はない。
- `src/internal/attestation.rs::verify_attestation` は `requires_tee` で分岐する。TEE モードでは小文字 hexadecimal の implicit account を要求し、その account に結びつく report data、現在時刻、owner が承認した measurement と PPID を使って attestation を検証する。
- local モードでは owner による `whitelist_agent_for_local` と、デフォルトの measurement / PPID の承認が必要。attestation はダミーであり、local は本物の TEE ではない。ホワイトリスト入りの後にも `register_agent` が必要。
- 登録成功時は `agents` に measurement、PPID、`valid_until_ms = 現在のブロック時刻(ms) + 有効期間` を保存し、`AgentRegistered` を出す。
- `src/internal/helpers.rs::require_valid_agent` は未登録なら `Agent not registered` で panic。登録済みでも、現在時刻が有効期限を超えている、measurement / PPID が承認解除されている、local ホワイトリストから外れている場合は無効とする。有効期限と現在時刻が同じなら有効。
- 無効な登録は削除して `AgentRemoved` を出し、private な `fail_on_invalid_agent` を呼ぶ Promise を返す。呼出元がこの Promise を返すことで、削除を維持しつつ後続 receipt を失敗させる。`get_agent` / `get_agents` は同じ判定理由を表示するが、登録自体は削除しない。
- 承認・解除・ホワイトリスト操作などは `require_owner` が predecessor と owner を照合する。登録されているだけでは、現在も有効とは限らない。

## Fixseal の追加 API

`src/your_functions.rs` に追加。`request_signature` と既存の認可処理は変更していない。

| メソッド | 引数 | 挙動 |
| --- | --- | --- |
| `submit_fix_proof` | `bundle_hash: String`, `summary: String` | `require_valid_agent` を通った agent のみ保存。同じ文字列の hash は全 agent 共通で重複拒否。成功時の JSON 戻り値は `null`、登録後に無効になった agent には既存の失敗 Promise を返す。 |
| `get_fix_proof` | `bundle_hash: String` | 未登録なら `null`、存在すれば記録を返す。認可不要の view。 |
| `list_fix_proofs` | `from_index: Option<u32>`, `limit: Option<u32>` | 挿入順。省略 / `null` では先頭から全件。範囲外または limit がゼロなら空配列。認可不要の view。 |

記録は `bundle_hash`、`caller`、`block_timestamp_ms`、`summary`。caller は署名元の `signer_account_id` ではなく、直接呼び出した `predecessor_account_id`。時刻は契約が取得するミリ秒の `U64` で、JSON では文字列になる。view は agent の後日の失効後も記録を返す。

local でもホワイトリストだけの書込みは許さず、テンプレートと同じ登録・有効性判定を要求する。`src/index.ts` の既存起動ループは whitelist 後に登録するため、この流れに一致する。

証跡は `IterableMap<String, FixProof>` に保存する。専用キー `FIX_PROOFS` に map のメタデータ、既存 enum の末尾に追加した `StorageKey::FixProofs` に要素を保存し、書込み時に flush する。既存 `Contract` の Borsh 状態形式と既存 storage key の番号は維持する。テンプレートの更新テスト用 WASM が旧状態を読むため、この構成にした。更新処理の実動作は未検証。

SDK の永続コレクションは要素を遅延ロードし、`flush` では変更した要素を書き込む。`FIX_PROOFS` へ全記録を再保存する方式ではない。R2 の **仮想空間での検証**では、1・10・100件のどれでもメタデータは16バイトで、最初と最後の記録の参照・挿入順も確認した。実チェーンの gas を測定した結果ではない。

この関数は non-payable。記録のストレージ費用は契約残高が負担する。文字列長・総件数の上限や費用回収はなく、登録済み agent による容量消費を制限しないデモ用 API である。hash は渡された文字列をそのまま扱い、形式・正規化、バンドルの内容、署名、攻撃結果の照合は T001 では行わない。これらは後続のパイプライン / verify CLI の作業。

## 検証状況

`tests/fix_proof_tests.rs` は以下の 11 テスト。NEAR SDK の mocked VM を使う**仮想空間での検証**用で、R2 で全11件成功。実 TEE、NEAR testnet、sandbox の receipt 実行を検証した結果ではない。

- 登録済み agent の成功、caller・時刻・summary・JSON 時刻の確認。
- 未登録の拒否。
- local ホワイトリストだけで未登録の呼出しの拒否。
- 別の登録済み agent による同一 hash の上書き拒否と元の記録の維持。
- 未存在の参照、挿入順、ページング、範囲外、limit ゼロ。
- 状態の保存・再読込み後の参照。
- 100件までの記録追加でメタデータの長さが一定で、記録の参照・挿入順が維持されること。
- 有効期限切れの agent の拒否と登録削除。
- measurement 承認解除後の拒否と登録削除。
- PPID 承認解除後の拒否と登録削除。
- local ホワイトリスト解除後の拒否と登録削除。

R1 では既存キャッシュを使い、契約単体51件・fix_proof10件・summary形式1件が成功。全契約テストの sandbox 使用9件は localhost bind の EPERM で失敗。運用者の R1 修正前のホストデモは全5版で記録・読み戻し・verify が成功している（[記録](demo-output-host.txt)）。無効 agent の Promise の最終的な receipt 失敗も新規テストでは実行しない。[BLOCKED.md](BLOCKED.md) に再開手順を記載する。
