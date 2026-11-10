# Codex (gpt-6.1-sol): blind spend repair

## English summary

**simulated environment, synthetic data**. In the recorded blind run, the agent worked in a copy without the reference answer, with UInt64/Bool-only Target arithmetic and no network access. Attempt 1 passed all behavioral checks (attacks 0/12, regressions 16/16) but was rejected with `PROOF_REJECTED` because of an unfinished equivalence in the over-limit branch. After reading Lean's error, attempt 2 was accepted; the kernel checked the proof against the fixed specification and the fixed tests were rerun. The final bundle replay reported `VERIFIED`; no chain run was performed.

The [archived attempts](../../examples/fixers/README.md) are post-experiment records. Attempt 2 is transcribed from the final source below. Attempt 1's complete original source was not retained in this record: its Proof is reconstructed from the final source, the described edit and the quoted diagnostic. It is not a byte-identical historical artifact. T008 locally rechecks both attempts with judge `0.1.0/lean-spend-2`; this is a replay, not a new blind run.

# FX3 修正記録（codex）

## 最終結果

仮想空間での検証。judge `0.1.0/lean-spend-2` の最終 verdict は `accept`、理由コードは `[]`、ポリシー違反は `[]`、`proof.verified = true`。

| 項目 | 結果 |
| --- | --- |
| 修正前の攻撃成功 | 12/12 |
| 修正後の攻撃成功 | 0/12 |
| 元の回帰テスト成功 | 16/16 |
| バンドル再検証 | VERIFIED、exit 0 |
| 固定仕様 SHA-256 | `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe` |
| 最終入力 SHA-256 | `58432c632bfba897bbb96f2bfe2a0de51ef87d393b1fa80fc2c0385526326812` |
| 結果 SHA-256 | `f6a9e248325312e81e6573116a4fa0a74b8b5d38daaf97f2c215ed3d0ed4cc19` |

Lean カーネルが固定仕様に対する証明を検査し、判定器が固定の攻撃・回帰テストを再実行した。鍵管理、決済、並行更新等の実運用については検証していない。チェーンへの記録・読み戻しは実施していない。

## 修正内容

`before/` を新しい `after_fx3/` にコピーした。加算による折り返しを避け、`spent ≤ limit` と `amount ≤ limit - spent` の両方を確認する。Target は UInt64 の比較・減算と Bool への判定のみを用い、他の数値表現へ変換しない。spent が上限内なら減算が数学的な自然数の減算に一致することを用い、両方向の同値を証明した。

候補内の変更は `Target.lean` と新規 `Proof.lean` のみ。`tests/regression.json` は baseline のコピーを保持し、バイト一致を確認した。その SHA-256 は `be5d23513c206ffd62d2753080718307582f4873d66a55e1d06fda027650995e`。既存の仕様・runtime・lakefile・judge・コーパス・ポリシー・既存テストは変更していない。

## 試行履歴

候補の修正・判定は2回。バンドル生成とその再検証は同じ最終候補の再確認で、追加修正はない。

1. 残額比較へ修正し、最初の証明を書いて指定コマンドを実行。`reject` / `PROOF_REJECTED`。攻撃成功0/12・回帰16/16・ポリシー違反なしだったが、証明の上限超過側で、最後の `omega` が `False ↔ spent.toNat + amount.toNat ≤ limit.toNat` を解消できなかった。判定JSONは Lean の診断を含まないため、同じ候補と固定仕様を `demo-artifacts/fx3/check/` で直接コンパイルして以下を記録した。

```text
Proof.lean:15:4: error: omega could not prove the goal:
a possible counterexample may satisfy the constraints
  c ≥ 0
  b ≥ 0
  a ≥ 0
  a - b ≥ 1
  a - b + c ≥ 1
where
 a := ↑spent.toNat
 b := ↑limit.toNat
 c := ↑amount.toNat
```

2. 上限超過側の同値を `constructor` で方向別に分けた。False からの方向は False の除去、逆方向は仮定した和の上限と spent の上限超過の矛盾を `omega` で示す形にした。Target は初回から変更なし。再度指定コマンドを実行し、`accept` / `[]` / `proof.verified = true`。攻撃成功0/12・回帰16/16。

判定前に、既にインストールされた Lean へ `#check` を実行して標準補題の型を確認し、比較を変換せず `omega` だけで証明する局所実験を1回行った。これは候補の判定回数には含めない。推測した未定義の補題名と未証明の局所目標について、以下のエラーを含む出力を得た。標準ライブラリのソースや別の解答ディレクトリは読んでいない。

```text
UInt64.toNat_sub (a b : UInt64) : (a - b).toNat = (2 ^ 64 - b.toNat + a.toNat) % 2 ^ 64
UInt64.le_iff_toNat_le {a b : UInt64} : a ≤ b ↔ a.toNat ≤ b.toNat
demo-artifacts/fx3/Probe.lean:4:7: error(lean.unknownIdentifier): Unknown constant `UInt64.toNat_le`
UInt64.toNat_lt (n : UInt64) : n.toNat < 2 ^ 64
UInt64.toNat_lt_size (a : UInt64) : a.toNat < UInt64.size
UInt64.toNat_sub_of_le (a b : UInt64) : b ≤ a → (a - b).toNat = a.toNat - b.toNat
demo-artifacts/fx3/Probe.lean:8:7: error(lean.unknownIdentifier): Unknown constant `UInt64.sub_toNat`
demo-artifacts/fx3/Probe.lean:9:7: error(lean.unknownIdentifier): Unknown constant `UInt64.le_def`
UInt64.size : Nat
demo-artifacts/fx3/Probe.lean:14:2: error: omega could not prove the goal:
a possible counterexample may satisfy the constraints
  c ≥ 0
  b ≥ 0
  a ≥ 0
  a + b - c ≥ 1
where
 a := ↑spent.toNat
 b := ↑amount.toNat
 c := ↑limit.toNat
```

## 実行とチェック

Rust/Cargo 1.99.1 と Lean 4.34.1 のインストール済み実体を使用。Rust の実体ディレクトリ `/opt/homebrew/opt/rustup/bin` を PATH に加え、`FIXSEAL_LEAN_BIN` に既存 Lean 4.34.1 の bin を指定。タイムアウトは既定120秒で変更していない。

```sh
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean --subject spend lean after_fx3
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean --subject spend lean after_fx3
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/fx3/bundle.json
```

| チェック | 実結果 |
| --- | --- |
| 候補判定 | 2回目 exit 0、accept |
| 候補バンドル出力 | exit 0、最終判定JSONと一致 |
| 候補バンドル verify | exit 0、VERIFIED、real 5.57秒（仮想空間での検証） |
| judge cargo fmt -- --check | exit 0 |
| judge cargo clippy --locked --offline --all-targets -- -D warnings | exit 0 |
| judge cargo test --locked --offline | exit 0、2 passed / 0 failed / 0 ignored |
| lean/ の lake build / lake env lean SpecTests.lean | exit 0 |
| lean/targets/spend/ の lake build | exit 1、固定lakefileの参照先 after_honest/Target.lean・Proof.lean が存在しない |
| python3 scripts/check-spend-proof-leak.py --self-test | exit 1、after_honest/Proof.lean 不在による FileNotFoundError |
| agent-contract の cargo test --locked --offline --test fix_proof_tests | exit 101、manifest path `agent-contract/Cargo.toml` does not exist |

標準ビルド・漏れ検査・契約テストの失敗は、この試験コピーに必要ファイルがないことによる未解決事項として記録する。設定変更、参照版の復元、テストの弱体化等は行わなかった。候補の fresh build・固定仕様・型・公理検査・攻撃・回帰は判定器と verify が実施している。

ログとJSONは `demo-artifacts/fx3/` に保存: `attempt-01.json`、`attempt-02.json`、`proof-01.log`、`probe-01.log`、`bundle.json`、`verify.log`、`verify.stderr`、`fmt.log`、`clippy.log`、`judge-tests.log`、`path-lake.log`、`spend-lake.log`、`proof-leak.log`、`contract-tests.log`。

## 最終 Target.lean 全文

```lean
import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  decide (spent ≤ limit ∧ amount ≤ limit - spent)

end Target
```

## 最終 Proof.lean 全文

```lean
import Target

theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro limit spent amount
  simp only [Target.authorize, decide_eq_true_eq]
  by_cases h : spent ≤ limit
  · simp only [h, true_and, UInt64.le_iff_toNat_le,
      UInt64.toNat_sub_of_le limit spent h]
    have hs := UInt64.le_iff_toNat_le.mp h
    omega
  · have hs : ¬ spent.toNat ≤ limit.toNat := by
      intro hh
      exact h (UInt64.le_iff_toNat_le.mpr hh)
    simp only [h, false_and]
    constructor
    · intro hh
      exact hh.elim
    · intro hh
      omega
```

## 所要時間・作業範囲

- 開始時刻（初回ツールでの記録）: 2026-11-10T01:59:28+00:00。
- accept 出力時刻（ログの更新時刻）: 2026-11-10T02:00:54.387368+00:00。開始から約86秒。
- チェック・記録の完了時刻: 2026-11-10T02:02:57+00:00。開始から209秒。
- 変更・追加: `lean/targets/spend/after_fx3/Target.lean`、`Proof.lean`、baseline と同一の `tests/regression.json`、本記録、`docs/tasks/FX3.md` の「結果」。生成ログ・コンパイル中間生成物は ignored の `demo-artifacts/` と `.lake/` に保存。
- `docs/HANDOFF.md` と `docs/progress.md` は存在しないため、更新対象なし。
- 外部通信、Ollama 通信、インストール、commit、push、送信は実施していない。ネットや別の解答を探していない。インストール済みツールの実行以外に、リポジトリ外のソースは読んでいない。
- 既存の staged 作業を保持し、git index の操作は行っていない。

## T008: 公開用アーカイブと復元

初回 Proof は最終版の末尾の `constructor` による方向別処理を、記録の15行目の `omega` に戻して復元した。 初回 Target は記録の最終 Target と同じ。初回ソースの完全な同一性は確認できないが、現行 judge で初回 `PROOF_REJECTED`、2回目 `accept`、両回とも攻撃0/12・回帰16/16を確認するテストを追加した。最終 Proof は import・空行を含めて19行。独立した修正担当へ渡すコピーからは、参照解と本記録・アーカイブを除く。
