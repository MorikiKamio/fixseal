# DeepSeek V4.1 Flash via NEAR AI Cloud — NX1

## English summary

**simulated environment, synthetic data / 仮想空間での検証**. Model
`deepseek/deepseek-v4.1-flash` ran through NEAR AI Cloud in an answer-withheld
copy on **2026-11-10, 04:50–05:07 UTC** (logged 04:50:42–05:07:44: 1,022 seconds).
It used **120 turns**, **5,771,118 prompt / 85,891 completion tokens**
(5,857,009 total). It ran the judge through `bash` and obtained `accept`,
but **never called `done`**. The wrapper stopped at `max_turns` and logged
**attempts 0**; that counter counts `done` submissions, not shell judge runs.
The operator reran the judge in the same copy and confirmed `accept`, attacks
**0/12**, regressions **16/16**, proof verified, fixed spec
`e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe`,
inputs SHA-256 `aeb2d11c25af0a1f7b6aff701d450fb20f0ea97e61ef74dd2028fa44be11c6f7`.
These are **spend-3** historical results, not spend-4 hashes or a chain run.
The Lean kernel checked a proof against the fixed specification and fixed tests
were rerun. The author of Fixseal is **Moriki Kamio**.

This differs from the two FX3 runs: a 120-turn cap, API transport, and operator
rejudging after an unsubmitted completion. It is not a controlled model ranking.
The supplied logs contain no `/v1/models` pricing snapshot or cache usage
breakdown; **historical USD cost is unavailable**. No cache discount is assumed.
T013 adds pricing capture and a separate `final_judge` fallback for future runs;
those changes were not used in NX1, and no new API run was made.

## Transcript extraction

Turns count assistant responses from 1; line numbers refer to the supplied
`transcript.jsonl`. The original transcript and operations logs remain local;
only this curated record is exported. Rows below are observed shell results,
separate from the wrapper's zero `done` attempts.

| Turn | Transcript command/result lines | Subject/version | Observed verdict | Attacks / regressions |
| --- | --- | --- | --- | --- |
| 29 | 74 / 75 | spend / after_nearai | accept, `[]`, proof=true | 0/12, 16/16 |
| 32 | 81 / 82 | spend / after_nearai | accept, `[]`, proof=true | 0/12, 16/16 |
| 32 | 81 / 83 | loop over other fixtures | command rejected by wrapper (variable path) | no judge verdict |
| 33 | 84 / 85 | loop over other fixtures | command rejected by wrapper (variable path) | no judge verdict |
| 34 | 86 / 87 | spend / before | reject, `ATTACK_STILL_WORKS` | 12/12, 16/16 |
| 34 | 86 / 88 | spend / after_saturate | reject, `ATTACK_STILL_WORKS` | 3/12, 16/16 |

The first candidate judge submission accepted after local Lean compilation
errors and revisions; it is not a claim that the first draft compiled.
Later record-writing commands quote judge commands/results and are not additional
observed judge executions. The extracted candidate source hashes match the
archived final files below. Baseline regression SHA-256 is
`be5d23513c206ffd62d2753080718307582f4873d66a55e1d06fda027650995e`.

## Current replay

[Archived final source](../../examples/fixers/spend/nearai-deepseek-v4.1-flash/final/Target.lean)
and [proof](../../examples/fixers/spend/nearai-deepseek-v4.1-flash/final/Proof.lean)
are byte-identical copies from the supplied blind copy. Run:

```sh
cargo test --manifest-path judge/Cargo.toml --locked --offline --test fixers nearai_recorded_final_replays
```

The test uses the current **spend-4** judge, fixed baseline regressions, expression
export/comparison on 1,028 fixed inputs and bundle replay. It expects accept,
0/12 attacks, 16/16 regressions and proof=true. The current input hash differs
from the historical spend-3 hash because the trusted exporter/version changed.
Exporter/interpreter correspondence is trusted, with fixed-input checks outside
the proof. This is an archive replay, not another blind run. No chain result,
TEE or affiliation with NEAR AI is claimed; using its Cloud API is the connection.

T013 measured a fresh no-chain bundle + verify in **16.041 seconds** on macOS
arm64 with installed toolchains and warm caches, alongside the fixture checks
(simulated environment, synthetic data / 仮想空間での検証). Verdict accept,
0/12 attacks, 16/16 regressions, proof=true, `VERIFIED`, 1,028 export checks.
With version label `after_nearai`, the current inputs SHA-256 is
`b8a1eae0597d8a42c51c78f4c62d2603d272ecab2305b701c2f1ce3729f80829`;
report SHA-256 is
`7fdd354dcb10d01c7e8f862ffe10d2c5d743e96b889b3c77d9bce476dff49e09`.
This timing excludes installation, API work and chain execution.

## Model-authored record

The following is the model's supplied report, retained as self-report. Its
“about 7 minutes” estimates time to acceptance, not the full 17-minute wrapper
run. Its “no external communication” describes model-invoked commands; the
wrapper did contact NEAR AI Cloud. Its count of two trials includes local
compilation and differs from both shell judge count and `done` attempts. The
operator-confirmed summary above takes precedence. Personal paths, if present,
are replaced with `<blind-copy>`.

---

# NX1: AI エージェントの支出上限チェック（spend）修正記録

作業者: nearai（修正担当 AI）
作業ディレクトリ: 準備済みリポジトリコピー（参照解答・after_honest 不在のブラインドコピー）
外部通信・インストール・commit・push なし。他ディレクトリの参照なし。

## 1. 最終結果

判定コマンド:

```sh
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean --subject spend lean after_nearai
```

| 項目 | 値 |
| --- | --- |
| verdict | accept |
| judge_version | 0.1.0/lean-spend-3 |
| validation_scope | 仮想空間での検証（simulated environment, synthetic data） |
| reason_codes | [] |
| policy_violations | [] |
| 攻撃 | 候補成功 0/12（baseline は 12/12 で脆弱性を再現） |
| 回帰 | 16/16 成功 |
| proof | verified: true, approved_spec_sha256: e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe |
| inputs_sha256 | aeb2d11c25af0a1f7b6aff701d450fb20f0ea97e61ef74dd2028fa44be11c6f7 |
| exit code | 0 |

補足（同じコマンドで再確認した他版。リポジトリ内の表と一致）:

- before: reject, ATTACK_STILL_WORKS, 攻撃 12/12
- after_saturate: reject, ATTACK_STILL_WORKS, 攻撃 3/12（飽和加算は上限最大で数学和の超過を拒否できない）
- after_hardcode / after_cheat / after_reject_all も reject

「kernel が固定 spec と固定テストに対する証明を検査し、元の回帰を再実行した」という事実のみを記録する。本修正が安全だという主張ではない。

## 2. 試行の回数と各回の reject と修正

ブラインドコピーには参照の after_honest/Target.lean と after_honest/Proof.lean が無いため自分で候補を書いた。判定器を回す前に安価に確認できるよう demo-artifacts/scratch/spend/（.gitignore 対象の作業用ディレクトリ）に固定入力（SpendSpec.lean, Main.lean, Audit.lean, TargetAudit.lean, lakefile.toml, lean-toolchain）をコピーして lake build を直接回した。scratch は判定前に削除した。

### 試行 0: 最初の着想（judge に出す前にローカルでエラーを検出）

before は decide (spent + amount ≤ limit) で、UInt64 の加算が 2 の 64 乗で巻き戻るため spent + amount が上限を超えても小さく見える（例: 0 18446744073709551615 1 が true）。方針は「巻き戻らない形に書き換える」= 引き算で判定する:

```lean
def authorize (limit spent amount : UInt64) : Bool :=
  spent ≤ limit && amount ≤ limit - spent
```

数学和 spent + amount ≤ limit と同値（spent ≤ limit の下で limit - spent は自然数の引き算になり、amount ≤ limit - spent ↔ spent + amount ≤ limit）。

最初に書いた証明（失敗）:

```lean
theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  constructor
  · intro h
    simp [Target.authorize] at h
    omega
  · intro h
    simp [Target.authorize]
    omega
```

Lean のエラー文:

```
error: after_honest/Proof.lean:8:4: omega could not prove the goal:
a possible counterexample may satisfy the constraints
  d ≥ 0
  c ≥ 0
  b ≥ 0
  b + c - d ≥ 1
where
 b := ↑s.toNat
 c := ↑a.toNat
 d := ↑l.toNat
error: after_honest/Proof.lean:11:4: omega could not prove the goal:
a possible counterexample may satisfy the constraints
  d ≥ 0
  c ≥ 0
  b ≥ 0
  b - c - d ≥ 0
where
 b := ↑l.toNat
 c := ↑s.toNat
 d := ↑a.toNat
```

原因: simp が spent ≤ limit を toNat の不等式に落とす一方、amount ≤ limit - spent に現れる UInt64 の引き算は omega では扱えない。(l - s).toNat を自然数の引き算に変換する補題 UInt64.toNat_sub_of_le を明示的に使えば omega に渡せる。

### 試行 1: toNat_sub_of_le を明示して成功（ローカル）

使用する補題を #check で確認（demo-artifacts/scratch/probe.lean、削除済み）:

```
UInt64.toNat_sub_of_le : ∀ (a b : UInt64), b ≤ a → (a - b).toNat = a.toNat - b.toNat
UInt64.le_iff_toNat_le : ∀ {a b : UInt64}, a ≤ b ↔ a.toNat ≤ b.toNat
```

- 順方向: h : spent ≤ limit ∧ amount ≤ limit - spent から hsl : s ≤ l と hal : a ≤ l - s を取り出し、hsub := UInt64.toNat_sub_of_le l s hsl で (l - s).toNat = l.toNat - s.toNat に直してから UInt64.le_iff_toNat_le.mp で自然数の不等式に落とし、omega で s.toNat + a.toNat ≤ l.toNat を得る。
- 逆方向: h : s.toNat + a.toNat ≤ l.toNat から omega で hsl' : s.toNat ≤ l.toNat、hsl : s ≤ l := UInt64.le_iff_toNat_le.mpr hsl'、hsub で引き算を自然数に直して hal : a ≤ l - s、最後に simp only [..., decide_eq_true_eq] で ∧ の証明に組み替えて exact ⟨hsl, hal⟩。

この時点で scratch の lake build は成功し、公理監査も 'checked_patch' depends on axioms: [propext, Quot.sound]（許可集合内）、TargetAudit.lean も FIXSEAL_TARGET_POLICY_OK を出力した。

### 試行 2: 最終候補を判定器で実行 → 一発 accept

lean/targets/spend/after_nearai/ を before/ からコピーし（tests/regression.json はバイト同一）、Target.lean と Proof.lean のみ編集した。最終形の Target.lean は decide を使う UInt64/Bool 演算のみの版。judge は reject 理由を 1 つも出さず accept。

集計: Lean のコンパイルエラーはローカルで 1 回（試行 0）。判定器（judge）が出した reject は 0 回。試行 2 回で accept。

## 3. 制約への対応

- Target.lean は UInt64 と Bool の演算のみ。toNat・Nat・BitVec・Fin・整数の数値射影・match / by・scope / 別名 / syntax 定義は不使用（policy.json の target_forbidden_identifiers、表現アクセス検査、TargetAudit.lean の elaborated body 検査に抵触しない）。

- Proof.lean は自由なので toNat と core 補題・omega を使用。sorry / admit / axiom / native_decide / bv_decide / implemented_by / extern / unsafe / opaque / set_option は不使用。追加宣言・import なし。

- SpendSpec.lean、Main.lean、Audit.lean、TargetAudit.lean、lakefile.toml、lean-toolchain、corpus.json、policy.json、回帰テストは無変更（judge の入力ハッシュでも baseline と candidate が一致）。

- 攻撃入力の直書きなし（contains_spend_literal が探す完全な数値三つ組は Target / Proof に無い）。

- 仕様を弱めていない。Correct の同値 authorize l s a = true ↔ s.toNat + a.toNat ≤ l.toNat をそのまま証明した。

## 4. 最終の Target.lean と Proof.lean 全文

### lean/targets/spend/after_nearai/Target.lean

```lean
import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  decide (spent ≤ limit ∧ amount ≤ limit - spent)

end Target
```

### lean/targets/spend/after_nearai/Proof.lean

```lean
import Target

/-- The candidate authorizes exactly when the mathematical sum fits in the limit.
`limit - spent` is defined whenever `spent ≤ limit`, and `toNat_sub_of_le` then turns it
into the ordinary natural subtraction, so the two side conditions are exact. -/
theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  constructor
  · intro h
    simp only [Target.authorize, decide_eq_true_eq] at h
    obtain ⟨hsl, hal⟩ := h
    have hsl' : s.toNat ≤ l.toNat := UInt64.le_iff_toNat_le.mp hsl
    have hsub : (l - s).toNat = l.toNat - s.toNat := UInt64.toNat_sub_of_le l s hsl
    have hal' : a.toNat ≤ l.toNat - s.toNat := by
      rw [← hsub]
      exact UInt64.le_iff_toNat_le.mp hal
    omega
  · intro h
    have hsl' : s.toNat ≤ l.toNat := by omega
    have hsl : s ≤ l := UInt64.le_iff_toNat_le.mpr hsl'
    have hsub : (l - s).toNat = l.toNat - s.toNat := UInt64.toNat_sub_of_le l s hsl
    have hal' : a.toNat ≤ (l - s).toNat := by
      rw [hsub]
      omega
    have hal : a ≤ l - s := UInt64.le_iff_toNat_le.mpr hal'
    simp only [Target.authorize, decide_eq_true_eq]
    exact ⟨hsl, hal⟩
```

lean/targets/spend/after_nearai/tests/regression.json は before/tests/regression.json とバイト同一（sha256 be5d23513c206ffd62d2753080718307582f4873d66a55e1d06fda027650995e）。

## 5. 掛かった時間

探索開始から最終 accept まで約 7 分（13:50 開始、13:57 終了）。概算の内訳:

- 仕様・policy・corpus・judge ソース（judge/src/lean.rs の検査、judge/src/lib.rs の採点、Main.lean）の読解: 約 3 分

- scratch プロジェクト準備と Lean のビルド 2 回（エラー 1 回 → 修正 1 回）: 約 2 分

- 候補作成と judge 実行（accept）+ 他版の再確認: 約 2 分

## 6. 限界と未確認事項

- 固定 spec ハッシュと判定器の一致は、このコピーに pin された値に依存する。SpendSpec.lean 内容の人間承認履歴はリポジトリ内に無い。

- 攻撃 12 件・回帰 16 件は固定コーパスに対する結果で、任意の難読化への耐性や汎用サンドボックス性を保証しない。

- Rust 実装と Lean のブリッジ証明（Aeneas 等）は対象外。Main.lean の入出力層は証明範囲外。

- WASI / TEE 実行は未確認。
