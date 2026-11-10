# Proof scope and trust assumptions

Fixseal reports a **simulated environment, synthetic data**. The Lean kernel checks a proof against a fixed specification and the judge reruns fixed tests. This does not establish that arbitrary code or real filesystem access is secure. Local attestation is mock.

A later [operator host result](docs/t010-host-check.md#host-result-operator-2026-11-10-1420-jst) reports a current-source wasm rebuild and successful local guard sandbox payments/key checks. The T013 coding task inspected the supplied logs offline; it did not rerun that build or chain execution. Historical statements below describe the earlier tasks. On 2026-11-10 the guard scenario also ran on NEAR testnet with disposable accounts and mock attestation ([run](docs/testnet-run-2026-11-10.md)). Real TEE and WASI remain unconfirmed.

For path, `Spec.Correct Target.join` is the conjunction of `Contained` and `AcceptsRelative`. Every returned normalized path must have the allowed root as a prefix. For relative requests with ordinary root/request components (neither empty, `.` nor `..`), the function must return the normalized concatenation, including for empty lists. Rejecting everything therefore fails. The theorem quantifies over every request, including its `raw` field, but does not prove that raw strings correspond to parsed components, that components contain no slash, or that lexical normalization matches OS behavior.

For spend, `SpendSpec.Correct Target.authorize` requires, for every UInt64 input, acceptance exactly when the mathematical sum of spent and amount is within limit. It covers neither keys, currency units, concurrent state updates nor payments. Target policy combines source inspection with an audit of the elaborated definition: constants in the Nat, Int, BitVec and Fin families are rejected, including inferred type arguments and unused let bindings. UInt64 literals remain permitted. This audit inspects the definition's expression, not the transitive implementations of trusted UInt64/core functions. Proof arithmetic is unrestricted by this Target policy.

Trusted inputs include the reviewed specification, the judge, baseline, corpus, original regressions, policy, Lean 4.34.1 kernel and standard library/axioms, Lake, compiler/runtime, native C toolchain and host OS. Path parsing, URL decoding, JSON I/O, Rust reference arithmetic and hashing are outside the theorem. Symlinks, mounts, permissions, case sensitivity, Windows paths, TOCTOU and actual file I/O are outside the model. The fixed spec hashes are `f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b` (path) and `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe` (spend). An operator must review their content and hashes before real use; pinned hashes do not record human approval.

Only candidate Target/Proof files may change. Runtime/build inputs are compared with trusted bytes, and fresh temporary projects replace candidate build artifacts. Fixed wrappers, source restrictions, theorem types and an axiom audit permit only `propext`, `Quot.sound`, `Classical.choice`. This is a restricted source interface, not a general Lean sandbox. Each build/audit/execution subprocess has a default 120-second limit; positive integer `FIXSEAL_LEAN_TIMEOUT_SECS` overrides it. Timeouts produce `PROOF_TIMEOUT`, are not cached, and require matching settings on replay. CPU, memory and output limits remain outside this protection.

`verify` rechecks input/result hashes and the complete judge report. `VERIFIED` also applies to reproducible rejections: inspect the verdict. Optional chain hash/summary comparisons do not authenticate offline readback JSON, caller or timestamp. The contract stores authorized agent claims and does not run Lean. The operator's [FIX5 host record](docs/demo-output-host-fix5.txt), path-7/spend-3, completed all ten versions with matched readbacks/replays and demo exit 0. This task performed no chain run. Real TEE, WASI, testnet and supplied wasm/source correspondence remain unconfirmed.

The proof leak scanner checks all text files regardless of extension and rejects encountered symlinks, with documented generated/archive exclusions. It detects copied fragments and named hints, not all semantic paraphrases. Independent fixer copies must exclude all reference `after_honest` directories and the post-experiment `examples/fixers` and `docs/fixer` records. See [replay records](examples/fixers/README.md).

## T010 exported policy and spending guard

The proof is about Lean's `Target.authorize`. The contract evaluates the exported expression using `shared/policy_expr.rs`, the same source included by the judge. Both the fixed exporter and interpreter are trusted and outside the proof. Comparing actual Lean outputs on the 12 attacks, 16 regressions and 1,000 boundary triples is a fixed-input check, not a proof of correspondence. Unsupported exports are rejected; any comparison disagreement is EXPORT_MISMATCH. Anyone can rerun `fixseal-judge verify` to re-export, hash and check the bundle/proof with trusted reference inputs.

The contract trusts the authorized record submitter's verdict/spec/expression claims, without a TEE or verifier quorum (N=0). Owner policy installation checks accept/spend, pinned spec hash, canonical expression hash/content and allowlisted spend-4 judge. Full-access keys and owner code upgrades can bypass the policy and remain trusted. Restricted agent keys allow only guard_spend on the intended receiver, zero deposit; access-key gas allowance does not itself limit payment arguments or recipients. The contract's own funds pay recipients. Successful transfer callbacks update total_paid; pending reservations remain separate, and callback failure requires investigation. A formula replacement cannot recover past payments or reconstruct the true cumulative amount from wrapped spent. [API and limits](docs/guard.md), [host checks](docs/t010-host-check.md), [testnet evidence template](docs/testnet-evidence.md). T010 chain execution was unconfirmed at implementation time; see the later operator host result above.

## Japanese reference

# 証明の範囲と信頼の前提

Fixseal は**安全の証明ではない**。ダミーのパス処理を使う「仮想空間での検証」であり、実ファイルは開かない。運用者のホスト通常シェルでは 2026-11-10 に path-5 のローカル NEAR sandbox への記録・読み戻し・再検証が全5版 matched / exit 0 で完走済み（[記録](docs/demo-output-host-path5.txt)）。T008 の path-7 / spend-2 は運用者が全10版 matched / exit 0 を確認済み（[記録](docs/demo-output-host-t008.txt)）。FIX5 の path-7 / spend-3 も運用者が全10版 matched / demo exit 0 を確認済み（[記録](docs/demo-output-host-fix5.txt)）。エージェントの制限環境では bind 権限で停止する。local の attestation は mock であり、本物の TEE ではない。

## 証明する性質

`lean/Spec.lean` の `Correct Target.join`、つまり `Contained` と `AcceptsRelative` の両方を証明する。前者は任意の許可ルート（文字列要素のリスト）と入力について、返したパスの正規化結果が許可ルートを接頭辞に持つ性質。後者は root と request.parts が通常の要素（各要素が空文字・`.`・`..` のいずれでもない）のリストで、相対リクエストなら、必ず `some (normalize (root ++ request.parts))` を返す性質。空リストも含み、全拒否する実装は満たせない。空要素・`.`・`..` を含む入力や絶対パスの受理可能性までは証明せず、元の回帰テストも併用する。

入力は `absolute` と `parts`、デモの直書き修正を再現するための `raw` を持つ。証明はこれらの全値を量化し、特定のコーパスだけに限定しない。`raw` の内容、各要素に `/` がないこと、文字列と要素列の対応は証明しない。実行入口の `Path.parse` が一度の URL デコードと `/` による分割を行う。`Spec.normalize` は承認対象となる字句的意味論であり、OS の意味論と一致するという証明はない。

`after_honest/Proof.lean` は `sorry` と追加公理を使わない。`Audit.lean` は実行入口が呼ぶ `Target.join` と同じ定義に対して証明の型を固定する。現在の証明が依存する Lean 標準公理は `propext`、`Quot.sound`。judge の公理検査はこの2つと標準の `Classical.choice` のみを許可する。

## 信頼するもの

- 人間による spec の内容のレビューと固定。T004 の参照 SHA-256 は `f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b`。judge に固定し、入力された spec のハッシュと照合する。これは実装時の固定値であり、人間による承認の履歴を自動生成するものではない。運用前に判定者が内容とこの値を確認する。
- Lean 4.34.1 のカーネル、標準ライブラリ、公理、コンパイラ、実行時、Lake、ネイティブの C ツールチェーン。機械語までの正しさを証明してはいない。判定機の固定 toolchain と環境も信頼する。
- `Path.lean` の文字列分解・UTF-8 検査・URL デコード、`Main.lean` の JSON 入出力・パスの再構成、judge の実行・理由コード・ハッシュ・ポリシー検査。攻撃・回帰・追加境界テストで確認するが、これらは形式証明の外。
- OS とファイルシステム。symlink、mount、アクセス権、大文字小文字の扱い、Windows パス、TOCTOU、実ファイルの読み書きは対象外。字句的に内側でも実 OS で安全とは限らない。
- 判定者が固定する judge バイナリ、baseline、コーパス、回帰テスト、ポリシーとそのハッシュ。変種攻撃もこのデモでは公開されている。判定入力をすべて任意に差し替えてよいという契約ではない。

## 判定の制限

Lean モードはネイティブ専用。新規の一時ディレクトリで固定の補助コードを使って `lake build pathcheck` を行い、その実行ファイルに攻撃・元の回帰テストを流す。行動検査が成功した候補だけ `lake build` と公理監査を行う。既存の攻撃・回帰・ポリシー違反があれば、それらの理由を維持する。禁止事項の検出は先に行い、他の失敗があっても `PROOF_CHEATED` を付ける。

修正者が変更できる Lean ファイルは `Target.lean` と `Proof.lean` のみ。spec、実行入口、補助コード、監査コード、ビルド設定、toolchain 指定は固定値と照合する。入力されたビルド生成物は使わない。候補の Lean は固定宣言ラッパーと禁止トークン検査の下で core の式・一般的な証明タクティクを受理し、追加 import・macro・initialize・IO・unsafe・外部実装・`sorry`・`axiom`・`native_decide` 等を実行前に拒否する。構文制約は正しい一般の Lean プログラムも拒否する。任意の Lean を実行する汎用サンドボックスではない。

プロセス内のキャッシュはソースと実行入力の SHA-256 で区別する。カーネル検査のキャッシュも実装・証明の両方で区別し、候補から渡された `.olean` や実行ファイルは信用しない。キャッシュ、ビルドログ、一時パス、時刻は判定 JSON に含めない。Lean ビルド・公理監査・対象実行の各子プロセスには既定120秒の wall-clock 制限を設ける。`FIXSEAL_LEAN_TIMEOUT_SECS` の正整数で変更でき、不正値や0は120秒。超過は `PROOF_TIMEOUT` として reject し、timeout の結果はキャッシュしない。成功・その他の失敗のキャッシュキーにも制限値を含める。Unix ではプロセスグループを終了し、直接の子を回収する。これは資源上限全般を備えた汎用サンドボックスではなく、CPU・メモリ・出力量の上限は別途必要。再検証では同じ toolchain と制限値を使う。

環境に必要な toolchain がなければ reject となり、別環境でも無条件に accept が一致するという保証はない。

## T004 の verify とチェーン記録

`verify` はバンドル内の入力・判定・結果ハッシュの整合性を再計算する。Lean の禁止事項・固定コード・証明も再検査する。reject の記録も再現できれば VERIFIED となる。チェーンの読み戻しがある場合は結果ハッシュと verdict / judge version / 全体入力ハッシュの summary を照合する。オフラインの読み戻し JSON の存在・caller・timestamp の真正性は検証せず、RPC 接続もしない。判定入力と期待値を一緒に差し替えれば別の整合したバンドルを作れるため、信頼する judge と baseline / corpus / policy、結果ハッシュの出所を固定する必要がある。契約自体は証明や verdict を判定せず、認可済み agent の申告を保存する。

## T006 spend の証明範囲

`lean/targets/spend/SpendSpec.lean` の `Correct Target.authorize` は、全 UInt64 入力について、許可結果が true であることと `spent.toNat + amount.toNat ≤ limit.toNat` の同値を要求する。固定参照ハッシュは `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe`。証明用補題は置かない。固定仕様に対する証明を Lean カーネルが検査したことを示すものであり、人間による承認履歴、鍵管理・通貨単位・並行更新・実際の決済やウォレットの動作は範囲外。

spend の Main / Audit / Lake / toolchain も trusted bytes に固定し、判定時は候補由来のビルド設定・生成物を使わない。入出力変換と judge の u128 参照計算は証明の外で、固定テストで確認する。公理監査は従来と同じ3公理だけ許可する。題材を入力ハッシュとキャッシュキーに含め、verify は同じ題材で再実行する。spend のソース・文書から証明のコピーと固有の補題ヒントを検出するが、意味を変えて書き直したヒントまで完全に検出するものではない。独立修正担当の盲検実験は [FX3記録](examples/fixers/README.md) を参照。実験後の記録は漏れ検査から除外し、次の修正担当へ渡すコピーには含めない。

T007 の spend-2 は候補 Target の禁止識別子（名前空間・ドット記法を含む）と内部表現の分解を実行前に検査する。Proof は対象外。限定的なソースポリシーであり、一般の Lean の型・意味を完全解析するものではない。T008 では after_cheat の Target を UInt64/Bool のみに変更した。同版は攻撃0/12・回帰16/16でも、不正証明により PROOF_CHEATED のみで拒否する。

## T010の追加前提

証明対象はLeanのTarget.authorizeで、コントラクトは書き出した式を共有Rust解釈器で評価する。固定の書き出し器・解釈器は信頼対象であり証明対象ではない。1,028入力の突き合わせは固定入力での検査であり証明ではない。記録の提出者を信頼し、TEE・検証者quorumは使わない。誰でもfixseal-judge verifyで再検査できる。所有者・full-access鍵・コード更新権限は信頼し、function-call keyのreceiver/method制限とガス枠は引数の送金額や宛先を制限しない。過去の送金は式の差し替えでは取り消せない。実送金とcallbackの確認はホストで行う。
