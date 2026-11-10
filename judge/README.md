# Judge: English evaluation guide

The local judge evaluates synthetic path and spending examples: **simulated environment, synthetic data**. Use installed Rust/Cargo 1.99.1, Lean 4.34.1, a native C compiler and cached locked dependencies. No account, chain, Ollama or external service is needed. Run from the repository root:

```sh
mkdir -p demo-artifacts
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean lean after_honest > demo-artifacts/bundle.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/bundle.json
```

For spend insert `--subject spend` after `--lean` during bundle generation. Replay reads the subject from hashed inputs. Expected honest verdict: `accept`, reasons `[]`, proof verified `true`; replay ends `VERIFIED`. Reproducible rejected bundles also verify: inspect the verdict. Direct judging exits 0 for accept, 1 for reject, 2 for malformed CLI/input; bundle export exits 0 even for reject. The VM-only historical route is `cargo run --manifest-path judge/Cargo.toml --locked --offline -- target after_honest`.

Reports contain judge version, validation scope, verdict/reasons, per-input and aggregate SHA-256, fixed attack/regression counts, policy violations and proof/spec status. Result hashes cover canonical `Report::to_json()` bytes including the final LF. Bundles contain inputs/report/result hash and optional chain readback. `verify` recomputes hashes, rejudges, compares the complete report and checks optional hash/summary fields; it does not authenticate offline chain evidence.

Current Lean judges are path-7 and spend-4. Candidates may edit only Target/Proof and must retain baseline regressions byte-for-byte. The judge checks fixed spec hashes and trusted runtime/build bytes, uses fresh temporary projects, executes fixed attacks and original regressions, then checks the fixed theorem type and allowed axioms. Spend additionally audits the elaborated Target expression for Nat/Int/BitVec/Fin constants before execution; Proof may use these types. Trusted UInt64/core implementations are not recursively expanded. Source restrictions remain supplemental; attack-literal matching is not general obfuscation detection. All Lean subprocesses default to 120 seconds; use matching `FIXSEAL_LEAN_TIMEOUT_SECS` for replay. A timeout is `PROOF_TIMEOUT`, never proof success.

See [JUDGES](../JUDGES.md) for all reason codes, [AGENTS](../AGENTS.md) for candidate syntax, [ASSUMPTIONS](../ASSUMPTIONS.md) for exact properties/trust boundaries and [setup](../docs/SETUP.md) for prerequisites. Run the judge suite with `cargo test --manifest-path judge/Cargo.toml --locked --offline`. The historical implementation/format details follow.

Spend-4 adds a typed expression export from the elaborated Target and compares it with the shared Rust interpreter on 1,028 inputs. Reports/spend summaries include canonical JSON as a policy_expr string and policy_expr_sha256; replay re-exports and checks both. The new EXPORT_MISMATCH reason rejects a comparison disagreement. The fixed exporter/interpreter and the record submitter remain trusted; the Lean proof covers Target.authorize, while fixed-input correspondence is a check, not a proof. No TEE is used. [Wire format, guard and keys](../docs/guard.md).

## Japanese reference

# Fixseal judge

ダミーのパス・トラバーサルに対する、**この攻撃群・この判定プログラムでの結果**を出すローカル判定コア。合成データによる「仮想空間での検証」。契約、アカウント、Ollama、TEE、外部通信は使わない。

リポジトリ直下から実行する（Rust と依存キャッシュ、利用環境に合った PATH が必要）:

```sh
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo run --manifest-path judge/Cargo.toml --locked --offline -- target after_honest
```

最後の引数を `before` / `after_hardcode` / `after_weaken_test` に替えると reject を再現できる。stdout は改行付きの JSON 一つ。終了コードは accept が 0、reject が 1、読込・CLI・JSON転送形式エラーが 2（stderr に説明、stdout は空）。判定内容の形式違反は `POLICY_VIOLATION` の JSON になる。

## ターゲットと実行方式

`../target/` の各版に `path.fn` と `tests/regression.json` がある。修正前は URL デコード → ルートとの結合 → 正規化を行い、ルート外のパスを返してしまう。正直な修正では正規化後に許可ルート `/sandbox/files` 内かを確認する。hardcode 版は公開攻撃一つだけを拒否する。weaken_test 版は全入力を拒否し、正常系をテストから取り除く。

`path.fn` は小さなパス処理用の命令言語。Rust ソースのコンパイルや子プロセスを必要とせず、WASI でも入力されたソースを実行できるよう、Rust 製の固定インタープリタで評価する。版名で実装を選ぶ方式ではなく、ソースを読み、以下の命令を上から実行する。

| 命令 | 動作 |
| --- | --- |
| `decode_percent` | `%HH` を一度 UTF-8 デコード。不正なエンコード・UTF-8・NUL は拒否。 |
| `join_root` | 相対パスに許可ルートを前置。絶対パスはそのまま。 |
| `normalize` | POSIX の `/`、`.`、`..` を字句的に正規化。 |
| `require_within_root` | 正規化したパスが許可ルートと同じか、その子孫かを確認。 |
| `reject_exact 文字列` | 現在の文字列が完全一致すると拒否。 |
| `reject_all` | 全入力を拒否。 |
| `return_path` | 現在のパスを返す。最終命令として必須。 |

空行と `#` から始まる行はコメント扱い。命令数は最大64、ループ・ファイルアクセス・外部実行命令はない。パスは仮想的な文字列で、実ファイルを開かない。OS 固有の区切り、実ファイルシステムの symlink、二重デコードする別アプリの挙動はこのモデルの対象外。

## 入力と判定

ディレクトリ入力 `TARGET_ROOT VERSION` は以下を読む。絶対パスやディレクトリ列挙順、mtime は結果に含めない。

- `TARGET_ROOT/before/` の全ファイル: 信頼する修正前のソースと回帰テスト。
- `TARGET_ROOT/VERSION/` の全ファイル: 判定する修正案。追加ファイルも読み、差分とハッシュに含める。
- `TARGET_ROOT/corpus.json`: `public` と `variants` の攻撃文字列配列。両方とも非空が必須。
- `TARGET_ROOT/policy.json`: 完全一致の `editable_paths` / `immutable_paths` と `forbid_attack_literals`。
- `VERSION` の文字列。

現行ポリシーでは編集は `path.fn` のみ。`tests/regression.json` は編集不可。許可外の追加・削除・変更、必須ファイルの欠落を拒否する。候補のテストが変わっても、回帰判定には修正前のテストを使う。候補ディレクトリにコーパスやポリシーのコピーを追加しても、許可外の追加として拒否する。

攻撃文字列の直書き検査は、編集可能ファイルにコーパス内の文字列または一度デコードした文字列が含まれるかを調べる。Lean モードでは許可構文の文字列エスケープをデコードして比較し、攻撃を一度 URL デコード・`/` 分割した部品リストの直書きも検出する。部品を囲む括弧やコメントも同じ値として比較する（R2）。R3 では Target 内のリスト要素を文字列リテラルとそれを包む括弧に限定し、`let` / `if` 等の式・入れ子リストを含む候補を実行前に `PROOF_CHEATED` で拒否する（空リストは許可）。証明内の `simp [...]` は別の許可構文として扱う。任意の難読化や一般のプログラム解析を扱うものではない。修正前、コーパス、ポリシーは判定者が固定して与える信頼入力。これら自体を差し替えた判定は別の入力ハッシュの結果となる。

| 理由コード | 条件 |
| --- | --- |
| `VULN_NOT_REPRODUCED` | 修正前で全攻撃を再現できない（修正前ソースの形式不正を含む）。 |
| `ATTACK_STILL_WORKS` | 候補が一つでも正規化後に許可ルート外のパスを返す。 |
| `REGRESSION_FAILED` | 候補が元の回帰テストを一つでも満たさない。 |
| `POLICY_VIOLATION` | 差分・直書き・必須入力・入力内容の形式に違反。 |

理由は複数併記する。理由がない場合のみ accept。空の攻撃群・回帰テストでの accept は認めない。

## ハッシュと決定性

JSON に `judge_version`、`validation_scope`、`verdict`、`reason_codes`、`input_hashes`、`inputs_sha256`、件数、ポリシー違反を出す。バージョンは `0.1.0/path-vm-1`。実装・意味を変える際はバージョンも更新する必要がある。

`input_hashes` は名前順のマップで、各入力の生バイトに SHA-256 を適用する。名前は `baseline/相対パス`、`candidate/相対パス`、`corpus.json`、`policy.json`、`version`。JSON 入力ファイルの空白や末尾改行もハッシュに含まれる。空ディレクトリやファイルメタデータは入力にしない。

全体の `inputs_sha256` は、`fixseal-inputs-v1` + NUL を前置し、名前順に `名前のバイト長(u64 big endian)`、`名前(UTF-8)`、`内容のバイト長(u64 big endian)`、`内容` を連結した SHA-256。名前・長さも含めるため、内容の区切りやファイル名の変更も反映される。外部時刻・乱数・ネットワークは参照せず、JSON のフィールド順・配列順・末尾改行を固定している。

`--stdin` でも実行できる。`input::Bundle` の JSON（`version`: 文字列、`baseline` / `candidate`: 相対ファイル名からバイト配列へのマップ、`corpus` / `policy`: バイト配列）を与える。ハッシュの対象はバンドル内の生バイトで、転送用 JSON 自体の空白やキー順ではない。同じ入力をディレクトリ経由・stdin 経由で与えると、出力も同じになる。

## WASI と検証状況

```sh
cargo build --manifest-path judge/Cargo.toml --target wasm32-wasip2 --locked --offline
```

WASI 固有でない Rust の標準入出力とファイル読み込みを使う。WASI では `--stdin` にバンドルを渡すか、ディレクトリを preopen する実行形態を想定する。2026-11-10 時点の環境ではターゲット標準ライブラリ不足（E0463）でビルドが停止。導入は行っておらず、WASM のビルド成功・ランタイム実行・ネイティブとの出力一致は未確認。TEE 実行も未確認。

ローカルのホスト実行記録は [demo-output-host-path5.txt](../docs/demo-output-host-path5.txt)、環境別の状態は [BLOCKED.md](../docs/BLOCKED.md) を参照。変種攻撃も再現用のダミーとしてこのリポジトリに公開しており、秘密の攻撃群ではない。

## T003: Lean に対する判定

T002 の CLI と JSON は維持し、`--lean` を指定した場合に証明を追加検査する。

```sh
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean lean after_honest
```

`before` / `after_hardcode` / `after_cheat` は reject。Lean ソースをクリーンビルドして実行し、元の攻撃・回帰・差分ポリシーに加え、固定 Spec の SHA-256、禁止構文、`lake build`、証明の型と公理依存を検査する。`PROOF_MISSING` / `PROOF_REJECTED` / `PROOF_CHEATED` を追加した。判定バージョンは `0.1.0/lean-path-7`、追加フィールド `proof` は検査成功の有無と固定 Spec ハッシュを持つ。

`--lean --stdin` も同じ Bundle 形式を使う。ライブラリでは `input::load_lean_bundle` と `judge_lean` を使う。baseline / candidate に共有の6ファイル（Spec、Path、Main、Audit、Lake 設定、toolchain 指定）を含め、実装・証明・回帰入力とともに全バイトをハッシュする。実行入力となるコーパス・ポリシー・版名を含め、honest の個別ハッシュは20件。ハッシュのフレーミングは T002 と同じ。

固定 Lean 4.34.1 とネイティブコンパイラが必要。証明がない before / hardcode は既存の行動・ポリシー違反の理由を優先する。禁止事項の検出時は行動検査に失敗しても `PROOF_CHEATED` を付ける。詳細、構文制限、3コマンドの手順は [lean/README.md](../lean/README.md)、保証の限界は [ASSUMPTIONS.md](../ASSUMPTIONS.md) を参照。Lean モードは子プロセスを使うネイティブ実装であり、WASI / TEE での実行は未対応・未検証。

## T004: オフラインのバンドルと verify

```sh
mkdir -p demo-artifacts
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean lean after_honest > demo-artifacts/bundle.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/bundle.json
```

ビルド済みなら `judge/target/debug/fixseal-judge verify <bundle.json>`。アカウント不要で、通信しない。VM 版は `bundle target after_honest`。`bundle` は reject の記録も正常な成果物として exit 0 で出力する。元の判定コマンドは引き続き accept=0 / reject=1。

証拠バンドルのフィールドは `format: "fixseal-bundle-v1"`、`mode: "lean" | "vm"`、`input: Bundle`、`report: 判定JSON`、`report_sha256`、任意の `chain`。結果ハッシュは judge の `Report::to_json()`（struct の固定順序、UTF-8、末尾 LF 1個）の SHA-256。バンドル JSON 全体やキー順・空白はハッシュ対象にしない。個別・全体入力ハッシュは既存形式を維持する。

`verify` は次を表示する。

- バンドル形式、全入力の個別ハッシュと全体ハッシュの一致。不一致なら実行前に停止。
- 再実行した verdict と、バージョン・理由・件数・spec を含む判定全文の一致。
- Lean の固定 spec / 補助コード / 禁止構文検査と `lake build`・公理監査。行動検査で reject された候補の証明も検査し、その失敗理由を表示する。
- 結果ハッシュの一致。`chain` があるときは `get_fix_proof.bundle_hash` と `summary` も一致確認。

`chain` は契約の読み戻しそのままの `{bundle_hash, caller, block_timestamp_ms, summary}`。summary は `{inputs_sha256, judge_version, verdict}` をキー名順・空白なしでシリアライズした JSON 文字列。デモは result SHA-256 を契約の `bundle_hash` に記録し、判定の要約を summary に記録する。記録の存在・caller・timestamp の真正性をこの CLI が証明するわけではない。信頼できる出所の読み戻しデータと judge バイナリ・参照入力を使用する。

exit 0=`VERIFIED`（accept / reject の再現一致）、exit 1=`VERIFICATION FAILED`（不一致）、exit 2=ファイル・JSON・引数エラー。証明のない hardcode、禁止事項のある cheat、回帰で落ちる reject_all の reject 記録も検証可能。reject 記録で `VERIFIED` と出ても修正の受理を意味しない。

Lean の実体 bin を別の場所に導入した場合は `FIXSEAL_LEAN_BIN` を指定する。自動取得する proxy へのフォールバックはない。[準備手順](../docs/SETUP.md)を参照。直書き検査の強化に伴い R1 で path-2 から path-3、R2 で path-4、R3 で path-5 に更新した。T005 で wall-clock 制限と `PROOF_TIMEOUT` を追加し path-6 に更新した。過去の path-2 / path-3 / path-4 / path-5 バンドルは判定全文が異なり、現行版での再検証は不一致となる。入力ハッシュ・Spec・VM 版は変更していない。

## Lean 子プロセスの時間制限（path-6）

ビルド・公理監査・対象実行の各子プロセスは既定120秒で終了する。`FIXSEAL_LEAN_TIMEOUT_SECS` は正整数の秒数を受け付け、不正値・0は既定値になる。超過は `PROOF_TIMEOUT` として reject し、`PROOF_REJECTED` とは区別する。Unix ではプロセスグループを終了して直接の子を回収する。timeout はキャッシュせず、他のキャッシュも制限値別に分ける。入力ハッシュに環境設定は含まれないため、バンドル再検証では同じ toolchain と制限値を使う。[FIX5 ホスト記録](../docs/demo-output-host-fix5.txt)は path-7 / spend-3 の全10版 matched / demo exit 0。

## T006: 複数題材と一般的な core 証明

既定の path は既存の配置・Spec・入力ハッシュを維持。構文受理範囲の変更に伴う判定版は `0.1.0/lean-path-7`。新規の `spend` は `lean/targets/spend/`、判定版 `0.1.0/lean-spend-3`。各題材は Spec ファイルと固定ハッシュ、実行・監査入口、Lake 設定、toolchain、コーパス、ポリシー、Target/Proof ラッパーを持つ。

```sh
mkdir -p demo-artifacts
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean --subject spend lean after_honest > demo-artifacts/spend.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/spend.json
```

ライブラリは `load_lean_subject_bundle(root, version, Subject::Spend)`。spend の Bundle は `subject: "spend"` を持ち、入力ハッシュにも subject を含む。path で省略したときは従来の転送形式・ハッシュ。`--lean --stdin` と verify は Bundle の題材を読み、指定がある場合の不一致は CLI エラー。VM に spend を渡すと reject。CLI の `--subject` は `--lean` の直後に指定する。

spend の Target は UInt64 / Bool の演算に限定し、policy の `target_forbidden_identifiers` を候補 Target のみに適用する。名前空間・ドット記法の各成分を検査し、コメント・文字列は除く。別表現への変換、scope / alias / syntax 定義、数値射影・パターン分解を実行前に POLICY_VIOLATION で拒否する。Proof の toNat は制限しない。after_cheat の Target は UInt64/Bool のみで、攻撃0/12・回帰16/16。不正証明により PROOF_CHEATED のみで拒否する。5版の accept / reject は不変。

spend のコーパスは既存と同じ public / variants の文字列配列で、各要素は `limit spent amount` の10進 UInt64。回帰の expected は文字列 `"true"` / `"false"`。u128 の数学的な和と照合して入力の妥当性を確認し、実際の Lean 関数をビルドして実行する。攻撃数12、回帰16。直書き検査は完全な数値3個の並びを検出し、共通の境界定数だけでは違反にしない。任意の難読化は検出対象外。

proof は一般的な core タクティク・補題名を受理する。追加宣言・import・危険な実行系・環境変更等は拒否し、固定 Spec/実行入口と定理の型、公理依存の監査を維持する。具体的な構文は [AGENTS.md](../AGENTS.md)。制限された source interface であり、汎用サンドボックスではない。キャッシュは題材でも分離する。

全5版の期待値は [README の表](../README.md#agent-spending-limit-overflow)。仮想空間での検証で honest だけ accept、saturate は最大上限で攻撃3件を許可するため reject。証明本文や固有のヒントは honest 以外に置かず、テストではファイルを読む。`python3 scripts/check-spend-proof-leak.py --self-test` を judge テストからも実行する。

`scripts/demo.sh` は path → spend、`--subject path|spend` で単独選択、`--check` は存在確認のみ。FIX5 の path-7 / spend-3 は[運用者がホスト確認済み](../docs/demo-output-host-fix5.txt)。外部通信禁止の作業では実行せず、明示的に loopback NEAR が許可されたホストで確認する。過去 path-6 バンドルも判定全文が異なるため現行版で再生成が必要。

## T010: spend-4 の書き出し

型推論後のTargetをTargetAuditで歩き、限定した式の木に書き出す。正準JSONはop/args順・空白と末尾改行なし・数値は10進文字列。そのSHA-256とJSON文字列をreportとspend summaryに含める。summaryは既存3項目にsubject/spec_sha256/policy_expr/policy_expr_sha256を追加し、キー名順でシリアライズする。path形式は維持。攻撃12＋回帰16＋境界格子1,000件で共有解釈器とLean実行を突き合わせ、不一致はEXPORT_MISMATCH。verifyは式も再書き出し・照合する。詳細は[ガード手順](../docs/guard.md)。旧spend-3のホスト記録は履歴であり、現行spend-4のチェーン検証ではない。

## Terminal verdict summary

Direct judging and bundle export show a plain verdict/reasons/attack/regression summary on stderr when it is a terminal. Use a leading `--summary` for captured output, e.g. `fixseal-judge --summary --lean --subject spend lean after_honest`. Stdout remains JSON, and noninteractive callers without the flag retain empty summary stderr. No ANSI color is used, so `NO_COLOR` is respected. Lean diagnostics may precede the summary. `verify` retains its own final `VERIFIED` / failure status.
