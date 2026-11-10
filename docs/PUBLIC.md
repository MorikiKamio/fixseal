# Public export procedure

**Distribute only a fresh, checked output of `export-public.py`. Never publish the development tree, a bulk git snapshot or an old export.** The development tree retains internal tasks, QA reviews, operations logs and local settings. Ignore rules are a secondary safeguard and do not remove indexed files or history. The public allowlist is authoritative; new candidates require explicit inclusion.

In the development copy, choose a new local directory for each submission:

```sh
python3 scripts/export-public.py demo-artifacts/submission-current
python3 scripts/export-public.py --check demo-artifacts/submission-current
```

Existing output is never overwritten. Run `--check` immediately before distribution; after source changes, create a new export. It compares the exact allowlist, all current bytes after wasm sanitization, and rejects extra/missing files, symlinks and stale content with exit 2. Export checks relative paths, symlinks, private home paths and internal identifiers before creating output. Neither command commits, pushes or sends data.

Included files are implementation, tests, pinned Lean inputs, synthetic targets, lockfiles, notices and reproduction documents. Internal task/QA/design/progress/HANDOFF files, raw operations logs, local agent settings, secrets and build caches are excluded. Explicitly allowlisted host evidence includes [FIX5](demo-output-host-fix5.txt): operator run on 2026-11-10, path-7/spend-3, both Lean builds and ten matched readbacks/replays, demo exit 0. The operator reports 66 judge tests and 72 contract tests passing separately from this demo-only log. The earlier [T008 host record](demo-output-host-t008.txt) remains available as historical evidence. Reference evidence is preserved unchanged.

Two upstream test wasm files are sanitized only on export: 72 home-path occurrences in data/custom sections are replaced at the same length. Code, offsets and original files are preserved; execution of sanitized wasm remains unconfirmed. The existing SBOM also has a sanitized upstream path. This procedure does not erase information from the development copy, index or history.

Post-experiment `examples/fixers` and `docs/fixer` records are public, including clearly marked reconstructed first attempts. They contain answers. Independent fixer copies must exclude both directories and every reference `after_honest` directory. Scanner exclusions do not authorize shipping answers in a blind copy.

License inventory remains incomplete (R5-C05): CC-BY-4.0 full text/attribution for the distributed version, LLVM-exception dependency/NOTICE/count reconciliation, current judge/transitive dependencies, Lean/toolchain distribution scope and wasm/source provenance need review. The LLVM exception text already exists; do not describe it as missing. Existing LICENSE/NOTICE/third-party notices are retained. No external license retrieval was performed. The detailed historical inventory follows.


T011 adds the one-command reviewer script/helper/tests, the separate pinned
review Dockerfile/entrypoint, its ignore file, the review guide, two curated
path-7/spend-4 offline bundles and their evidence manifest. All are explicitly
allowlisted. Submission drafts/payload, task results and progress stay internal.
The bundles contain answers and live under the already excluded
`examples/fixers/evidence/` post-experiment archive. Keep the existing
post-experiment/reference exclusions for an independent fixer copy. A1/A2
check saved integrity only; C4 reruns the proof and fixed tests. CI and Docker
remain unexecuted, with a first-run placeholder in the READMEs.

T013 adds the standard-library `fixers/nearai/` tool and offline tests, the
CLI summary test, and the curated NX1 public record/final source archive (eight
explicit paths). Raw API transcripts, judgement logs, local paths, task results
and submission drafts stay internal. NX1 ran on spend-3 and did not call done;
current tests replay the exact final sources on spend-4. The fixer must run only
against an answer-withheld copy; a public export includes answers. Unit tests
use temporary synthetic copies, without sockets or API calls. Historical pricing
was not retained, so no NX1 USD amount is exported.

## Japanese reference and license inventory

# 公開対象

開発作業コピーには内部タスク・QA原票・運用記録が残っています。作業コピー全体を提出せず、開発作業コピーの `scripts/public-files.txt` で指定したファイルだけを公開用ディレクトリへ書き出します。

```sh
# 開発作業コピーで実行（外部送信はしない）
python3 scripts/export-public.py demo-artifacts/submission-current
python3 scripts/export-public.py --check demo-artifacts/submission-current
```

提出のたびに新しい出力先を指定し、提出直前に `--check` で現行ソースとの一致を確認してください。上の出力先が既にある場合も別名を使います。`--check` は許可リスト、全ファイルのバイト列（wasm 匿名化後）、余分なファイル、symlink を読み取りだけで検査し、不一致なら exit 2 です。ソース更新後は再生成が必要です。以前から残る `submission/` は古い版のため、そのまま提出しないでください。

既定の出力は ignored な `submission/` です。既存ディレクトリは上書きしません。再作成時は別の新しい出力先を指定できます。書き出し前にファイルの存在、相対パス、symlink、個人ホームパスと内部プロジェクト識別子の混入を検査します。

含めるものは実装、テスト、固定 Lean 入力、ダミーターゲット、lockfile、ライセンスと再現手順です。内部の tasks / qa / progress / HANDOFF / design、運用用シェルスクリプト、生ログ、ローカル agent 設定、秘密の `.env`、ビルドキャッシュは含めません。運用者の成功記録は個人パスだけを `<repo>` に置換し、版・出所を README に明記して含めます。公開書き出しツール・許可リスト・そのテスト自体は開発作業コピーの管理用です。

wasm テスト資材2個には上流開発者のホームパスがデータ・customセクションに埋め込まれているため、書き出し時だけ同じ長さのビルド用パスに匿名化します（計72箇所）。元ファイル、コード・その他セクション、ファイル長とオフセットは維持します。匿名化した資材のチェーン実行は bind 制限により未確認です。

SBOM の上流開発者の絶対パス1箇所も匿名化しています。既存 SBOM / 第三者ライセンス一覧は上流テンプレートを対象とした資料で、judge / Lean を含む完全な現行 SBOM ではありません。第三者ライセンス一覧の不足警告は未解消です。既存の LICENSE / NOTICE と帰属表記は維持しています。

公開用ディレクトリはローカル成果物です。この操作は git index の変更、commit、push、外部送信を行いません。元の作業コピー・既存 git 履歴から内部情報を消去する操作でもありません。

T008 の公開対象には `docs/fixer/` の2記録、`examples/fixers/` の各試行と説明、再判定テストも含めます。個人パスを匿名化した実験後の記録であり、初回 Proof の復元と原本同一性の限界を明記します。修正担当用の盲検コピーを作る際は、`examples/fixers/`、`docs/fixer/`、すべての参照 `after_honest/` を必ず除いてください。公開用コピーは解答を含むため、そのまま修正担当へ渡せません。漏れ検査の記録ディレクトリ除外は、盲検コピーへの同梱を許可するものではありません。

内部パスは開発用 `.gitignore` でも除外します。ただし既に git index に入ったファイルには効きません。現在の index 状態にかかわらず、作業コピーのまま commit / push せず、検査に通った公開用ディレクトリの中身だけを別の公開用作業コピーへ移す運用にします。ignore の追加で既存 index や履歴から消えたとは扱いません。

## T005: 第三者ライセンス・帰属の未完了項目

外部取得・インストールをせず、既存一覧・SBOM・ローカルキャッシュだけを確認しました。以下は配布準備の残件です。本文を推測して補完したり、棚卸し完了と扱ったりしません。

| 対象 | 確認できた状態 | 不足・次に必要な確認 |
| --- | --- | --- |
| `caniuse-lite` 1.0.30001727 / CC-BY-4.0 | 既存 SBOM に1件。`LICENSE-THIRD-PARTY.txt` の節12は概要と原文URLだけ。ルートと frontend の node_modules に対象 LICENSE はない。 | 正式な完全原文、当該版の配布物の帰属・追加通知。外部取得を許可された環境で該当版の配布物と照合する。 |
| Apache-2.0 WITH LLVM-exception | 一覧の節13には Apache 本文と LLVM 例外2段落が既にある。ローカル Cargo キャッシュの `wasmtime` 36.0.9 / `cranelift-codegen` 0.123.9 / `target-lexicon` 0.13.5 の LICENSE にも例外本文がある。冒頭の「used by 0 / missing」はこの存在と整合しない。 | 本文の不在とは扱わない。SBOM の式で対象になる依存と版ごとの LICENSE / NOTICE / 著作権表記の対応、実際の配布対象、集計処理との整合確認が不足。警告だけを削除しない。 |
| LLVM 例外の対象範囲 | 既存 SBOM は Cranelift / Wasmtime 群、pulley、regalloc2、target-lexicon、wasm-encoder、wasmparser、wasmprinter、rustix 等で式を宣言する。一部は MIT / Apache との選択式。 | 「0件」として除外せず、lockfile と実際の配布資材を突き合わせた全件の帰属一覧が必要。既存 SBOM の集計だけで現行配布の対象と断定しない。 |
| judge の Rust 依存と推移依存 | `judge/Cargo.lock` は存在するが、既存 SBOM は judge 追加前のテンプレート用。 | 現行 lockfile に対応する依存一覧、各版の原文・帰属・NOTICE の収集と第三者一覧への対応。ローカルの一部原文の存在だけで全件完了にはしない。 |
| Lean 4.34.1 / 標準ライブラリ / native toolchain | 使用版を固定しており、実行環境への持込みが必要。 | ソースだけの提出かバイナリ同梱かを確定し、配布対象に応じた原文・帰属・同梱物の棚卸しを行う。既存 SBOM はこれらを網羅しない。 |
| テスト用 wasm / 契約のビルド成果物 | 上流テスト用 wasm は既存通知を維持。個人パスの匿名化はコード・オフセットを維持する。 | 各バイナリと原典・ソース・生成版・帰属の対応、匿名化後の実行確認。持込み契約 wasm と現行ソースの対応も未確認。 |

README.ja.md / JUDGES.md / AGENTS.md / `.github/workflows/ci.yml` と path-5 のホスト参照記録を公開許可リストに追加しました。CI は今後の runner 用に依存取得手順を含みますが、本タスクでは外部アクセスや CI 実行を行っていません。
