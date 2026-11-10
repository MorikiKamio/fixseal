# Lean targets, specification and proofs

This project uses Lean 4.34.1 with core/Std only, no Mathlib or external Lean dependencies. Results describe a **simulated environment, synthetic data**. The same candidate function is executed in fixed tests and named in the theorem. Run from the repository root with native Lean binaries on PATH:

```sh
(cd lean && lake build && lake env lean SpecTests.lean)
(cd lean/targets/spend && lake build)
cargo test --manifest-path judge/Cargo.toml --locked --offline
python3 scripts/check-spend-proof-leak.py --self-test
```

A standalone build is insufficient: Lean can compile a `sorry` proof with a warning. The judge also checks source policy, trusted bytes, fixed hashes/theorem type, kernel proof and axiom dependencies. Allowed axioms are `propext`, `Quot.sound`, `Classical.choice`. Each subprocess has a default 120-second limit, configurable by positive integer `FIXSEAL_LEAN_TIMEOUT_SECS`; timeouts reject as `PROOF_TIMEOUT`.

Path lives here; spend is a separate project in `targets/spend`. Default Lake builds use `after_honest`. To add a candidate, copy to a fresh name, edit only its Target/Proof files, keep tests byte-identical and judge via `--lean lean VERSION` (insert `--subject spend` for spend). The judge compiles the named directory in a fresh project; names do not select hardcoded implementations. Never change the shared build configuration or spec to make a candidate pass.

Path's theorem is `Spec.Correct Target.join` (containment and acceptance of ordinary relative requests). Spend's is `SpendSpec.Correct Target.authorize` (acceptance iff mathematical sum is within limit). Spec hashes and precise trust boundaries are in [ASSUMPTIONS](../ASSUMPTIONS.md). Fixed shared code includes parsing/execution Main, proof Audit, Lake configuration and toolchain; spend also includes `TargetAudit.lean`, which rejects Nat/Int/BitVec/Fin constants in the elaborated Target body, types and let bindings before execution. UInt64 literals are allowed; Proof is exempt. It does not recursively audit trusted core implementations.

The fixed wrapper/import/theorem name and type must be retained. Core tactics such as `intro`, `cases`, `induction`, `rw`, `simp`, `omega`, `constructor`, `exact`, `apply`, `have`, `show`, `unfold`, `split`, `by_cases`, `decide` are supported. Extra declarations/imports, environment mutation, IO, unsafe/external code, `sorry`, new axioms and native decision tactics are rejected. This is a restricted source interface, not a general Lean sandbox. Path Target lists allow literal strings and parentheses only. See [AGENTS](../AGENTS.md) for exact wrappers and commands.

The proof leak scanner covers text of every extension and encountered symlinks, excluding specified build/generated directories and post-experiment archives. For independent fixer copies remove all reference `after_honest` directories, `examples/fixers` and `docs/fixer`. [Archived attempts](../examples/fixers/README.md) are replay evidence, not a new blind experiment. Current judges: path-7/spend-4. Historical details follow.

T010 spend-4 also exports supported elaborated UInt64/Bool expressions through TargetAudit. Unsupported fragments fail closed. The judge shares a Rust interpreter with the spending contract and compares it with Lean on a fixed 1,028-input set. The exporter/interpreter are trusted, outside the proof; comparison is a check, not a proof. [Expression and guard guide](../docs/guard.md).

## Japanese reference

# Lean の対象・spec・証明

T002 と同じダミーのパス・トラバーサルを Lean 自身で実装し、その関数をテスト・証明の両方の対象にする。結果はすべて**仮想空間での検証**。[ASSUMPTIONS.md](../ASSUMPTIONS.md) に保証の範囲と信頼の前提を記載した。

## 3コマンドで検査

リポジトリ直下で、Lean 4.34.1 と Rust の既存 toolchain / 依存キャッシュを使う。外部取得は不要。Rust の cargo / rustc を利用環境の PATH に置く。

```sh
export FIXSEAL_LEAN_BIN="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
export PATH="$FIXSEAL_LEAN_BIN:$PATH"
(cd lean && lake build)
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean lean after_honest
```

`lake build` は正直な修正版と証明、監査、実行ファイルをビルドする。最後の judge コマンドが spec 固定値・禁止構文・クリーンビルド・公理依存・攻撃・元の回帰テストを合わせて検査し、JSON で accept を出す。Lean 単体は `sorry` を警告だけで受け入れるので、**`lake build` 成功だけで受理とはしない**。

全テストは次で実行できる。

```sh
cargo test --manifest-path judge/Cargo.toml --locked --offline
```

## ファイルと修正範囲

| ファイル | 役割 |
| --- | --- |
| `Spec.lean` | 人間がレビューする仕様。要素リスト・字句的正規化・許可ルート内の定義・全入力の性質。修正者の編集は禁止。 |
| `after_honest/Target.lean` | 正規化後にルート内かを確認して返す関数。 |
| `after_honest/Proof.lean` | `Spec.Correct Target.join`（包含性 + 通常の相対入力の受理）の証明。`sorry`・追加公理なし。 |
| `before/Target.lean` | ルート外への移動を許す修正前。証明なし。 |
| `after_hardcode/Target.lean` | 公開攻撃の生文字列だけを拒否。変種を通す。証明なし。 |
| `after_cheat/` | 正直な実装と `sorry` を使った不正な証明。ビルドできても judge は拒否。 |
| `after_weaken_test/` | 正直な実装・証明と弱めた回帰テスト。元の回帰テストを使い、テスト改変を拒否。 |
| `after_reject_all/` | 常に none を返す実装。回帰で拒否。添付した honest の証明も新 spec に対して失敗。 |
| `SpecTests.lean` | 旧 spec で全拒否が通り、新 spec では通らないことを確認する補助例。`lake env lean SpecTests.lean` で検査。 |
| `Path.lean` / `Main.lean` | 一度の URL デコード・要素分割・JSON Lines 入出力。証明範囲外。 |
| `Audit.lean` | 実行する関数に対して証明の型を固定し、公理依存を出力。 |
| `corpus.json` / 各版の `tests/regression.json` | T002 と同じダミーの攻撃・回帰入力。judge は baseline の回帰入力を使う。 |
| `lakefile.toml` / `lean-toolchain` | Std/Init と Lean 標準の JSON のみ。外部依存・Mathlib なし。 |

デフォルトの Lake プロジェクトは `after_honest` の2ファイルを読む。各版の検査は最後の引数を `before`、`after_hardcode`、`after_cheat` に変更する。版名で実装を選ぶのではなく、指定されたファイルのソースをコンパイルする。Spec・実行補助・ビルド設定は共有の固定入力として、各版の入力ハッシュにも含める。各版内に同名の Spec を置いて上書きしようとしても検査に渡り、拒否される。

judge は `~/.elan/toolchains/leanprover--lean4---v4.34.1/bin/lake`（別の配置では `FIXSEAL_LEAN_BIN` 内の実体）を直接使い、elan の自動ダウンロードを避ける。候補が提供したビルド設定や生成物を実行しない。証明ファイルの import と定理の型を固定し、固定ラッパー・禁止トークン・公理監査を併用する。一般の Lean 修正を受け入れる仕組みではない。

Spec の参照 SHA-256 は `f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b`（`judge/src/lean.rs` に固定）。これはこの実装の基準値。運用時には人間が内容をレビューして承認し、judge バイナリとともに固定する。修正者による更新は許さない。

## 攻撃コーパスを直接実行

ビルド後の `lean/.lake/build/bin/pathcheck` は、stdin の各行に JSON の文字列を受け取り、返却パスの JSON 文字列または `null` を1行ずつ出す。実ファイルは開かない。次は T002 の攻撃11件を正直な実装へ直接流す例。

```sh
python3 - <<'PY' | lean/.lake/build/bin/pathcheck
import json
from pathlib import Path
corpus = json.loads(Path('target/corpus.json').read_text())
for attack in corpus['public'] + corpus['variants']:
    print(json.dumps(attack))
PY
```

すべて `null` になる。judge は同じ形式で各版を実行し、既存の Rust による字句的なパス判定で攻撃成功を数える。証明を Rust モデルに置き換えてはいない。

## 理由コードと期待値

以下は合成データによる**仮想空間での検証**。全版で修正前の攻撃成功11/11。元の回帰は reject_all が2/11、他は11/11成功。

| 版 | 判定 | 候補の攻撃成功 | 理由 |
| --- | --- | --- | --- |
| before | reject | 11/11 | ATTACK_STILL_WORKS |
| after_honest | accept | 0/11 | なし。カーネル検査成功。 |
| after_hardcode | reject | 10/11 | POLICY_VIOLATION, ATTACK_STILL_WORKS |
| after_cheat | reject | 0/11 | PROOF_CHEATED |
| after_weaken_test | reject | 0/11 | POLICY_VIOLATION |
| after_reject_all | reject | 0/11 | REGRESSION_FAILED（別途のカーネル検査は PROOF_REJECTED） |

T004 で `Correct = Contained ∧ AcceptsRelative` に強化した。`AcceptsRelative` は root と入力部品が空文字・`.`・`..` を含まない通常の要素列で、相対入力なら正規化した結合結果を必ず返す性質。空リストも含むため全拒否は通らない。空要素・`.`・`..` や絶対パスの受理可能性の証明は範囲外。回帰も引き続き使用する。

各 Lean ビルド・公理監査・対象実行は既定120秒の wall-clock 制限を持つ。`FIXSEAL_LEAN_TIMEOUT_SECS` に正整数の秒数を指定でき、超過は `PROOF_TIMEOUT` として reject する。現行 judge は `0.1.0/lean-path-7` と `0.1.0/lean-spend-4`。

行動検査を通った候補に証明ファイルがなければ `PROOF_MISSING`、許可構文でも型検査・ビルドに失敗すれば `PROOF_REJECTED`。`sorry`、独自公理、禁止タクティク、spec や実行補助の改変、未許可の Lean 構文は `PROOF_CHEATED`。既存の違反がある候補に証明なしの理由を重ねず、T002 の理由を維持する。

## 参考

設計上の参考: [NyxFoundation/nyxis](https://github.com/NyxFoundation/nyxis)（MIT）。人間が仕様を承認し、AI が実装・証明を書き、Lean カーネルが検査する役割分担と、信頼の前提を明記する作法を参考にした。設計 v0.3 で挙げられた [nyx-ssz](https://github.com/NyxFoundation/nyx-ssz)（MIT）の未証明範囲を明記する方針にも従う。ライセンス表記はタスク・設計資料に基づく。外部通信禁止のため本作業で取得・閲覧・コード転用はしていない。Rust と Lean の対応証明（Aeneas 等）は将来の拡張であり、今回の実装には含めない。

## 修正者向けの構文（T006）

宣言ラッパー・import・定理名と型は固定。一般的な core タクティクと core の補題名、任意の ASCII 仮説名、算術・比較・論理記法を使用できる。`rw` / `induction` / `omega` / `by_cases` / `decide` 等も利用可能。追加宣言、環境変更、メタプログラム、IO、unsafe、`sorry` / `native_decide` / `bv_decide` 等は禁止トークン検査で実行前に拒否する。詳細は [AGENTS.md](../AGENTS.md) と judge の `expression_allowed` / lexer。固定入力のバイト照合・Spec ハッシュ・固定型・カーネルと公理監査を維持する。

path の Target リストは従来どおり文字列リテラルと括弧のみ（空リスト可）。要素内の式・入れ子リストは拒否する。文字列は JSON 互換エスケープ、コメントは入れ子を認識する。直書き検査は補助的なポリシーであり、一般の難読化検出ではない。

## 支出上限の桁あふれ（spend）

spend の Target は UInt64 と Bool の演算に限定し、`target_forbidden_identifiers` で `toNat` 等の変換・別の数値表現・スコープや別名／構文定義（名前空間・ドット記法を含む）を、追加検査で内部表現の分解を `POLICY_VIOLATION` として拒否するが、Proof では `toNat` を自由に使える。

独立した Lake プロジェクトは `targets/spend/`。Mathlib・外部パッケージなし。`SpendSpec.lean` は `UInt64 → UInt64 → UInt64 → Bool` の承認関数について数学的な和が上限内であることと true の同値だけを定義し、証明用の補題を持たない。固定ハッシュは `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe`。内容とハッシュの人間レビュー履歴は未作成。

```sh
(cd lean/targets/spend && lake build)
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean --subject spend lean after_honest
python3 scripts/check-spend-proof-leak.py --self-test
```

入力は JSON 文字列内の空白区切り `limit spent amount`（10進 UInt64 3個）、出力は JSON 文字列 `"true"` / `"false"`。範囲外や形式不正は入力検査で拒否する。judge は数学的な和を u128 で判定してコーパスと回帰の妥当性も確認する。実行する関数は定理で指定するものと同じ `Target.authorize`。

合成データによる**仮想空間での検証**: 攻撃12件、回帰16件（許可9・拒否7）。before は攻撃12/12成功。honest は0/12・回帰16/16でaccept、hardcode は11/12でポリシー違反も検出、cheat は0/12・回帰16/16でも不正証明により PROOF_CHEATED のみで拒否、reject_all は回帰7/16、saturate は攻撃3/12でreject。飽和加算は最大上限で数学的な和の超過を拒否できない。

spend の参照証明本文と固有の補題ヒントは `after_honest` に置く。漏れ検査は他の版・ソース・文書を走査し、コピーを挿入する自己テストも行う。生成物と実験後の記録 `examples/fixers/`・`docs/fixer/` は走査外。独立修正担当に渡す際は、これらの記録とすべての参照 `after_honest` を必ず除く。[FX3記録と再判定手順](../examples/fixers/README.md) を参照。
