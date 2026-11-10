# Environment status: English summary

The operator's [FIX5 host record](demo-output-host-fix5.txt) completed path-7/spend-3 on 2026-11-10: both Lean builds, ten matched readbacks/replays and demo exit 0. The operator also reports 66 judge tests and 72 contract tests passing; full suite output is not part of the linked demo log. All results describe a **simulated environment, synthetic data**. This task does not run the chain.

Restricted agent environments may lack localhost bind permission and fail with `Operation not permitted`; this is not a completed chain test. Prepared offline Lean/judge and contract mock tests need no socket. Chain tests require preinstalled near-sandbox 2.11.0, matching caches, supplied contract wasm and explicit loopback permission. `scripts/demo.sh --check` checks file presence only. Use [SETUP](SETUP.md) for setup/wasm commands and [JUDGES](../JUDGES.md) for expected output. Never work around bind restrictions or download tools without authorization. WASI, real TEE/testnet, CI execution and wasm/source correspondence remain unconfirmed. Historical notes below retain their original context.

## Japanese reference

# 実行環境ごとの状態と再開条件

更新: 2026-11-10（T009）。初回準備、wasm 生成、オフラインテストのコマンドは [SETUP.md](SETUP.md) に集約しています。

## 検証の出所

- 運用者のホスト通常シェルでは、2026-11-10 に path-5 が `exit=0` で完走。[demo-output-host-path5.txt](demo-output-host-path5.txt) は全5版の deploy → whitelist/register → submit → get → verify が `matched` / `VERIFIED` の記録です。合成データによる**仮想空間での検証**です。
- 現行 judge は `0.1.0/lean-path-7` と `0.1.0/lean-spend-3`。後記の T006 ホスト記録（path-7 / spend-1、全10版 matched）は保全しています。[T008 ホスト記録](demo-output-host-t008.txt)で path-7 / spend-2 の全10版 matched / exit 0 を確認済み。FIX5 の path-7 / spend-3 も[運用者ホスト記録](demo-output-host-fix5.txt)で全10版 matched / demo exit 0 を確認済みです。今回は localhost Ollama 以外の通信禁止のため NEAR は起動していません。エージェント環境では localhost bind が `Operation not permitted (os error 1)` で失敗するため、同環境でのチェーン完走とは扱いません。
- judge のオフラインテストと Lean のビルド・仕様テスト、契約の mock 単体テストは既存キャッシュで実行できます。sandbox 統合テストを ignore に変えて成功扱いにはしません。

## バイナリ・wasm がない場合

デモは不足を表示して exit 2 で止まります。自動取得・インストールは行いません。`scripts/demo.sh --check` は存在確認のみです。必要物のバージョンと生成・持込み手順は [SETUP.md](SETUP.md) に記載しています。クローンだけで完全オフラインの初回ビルドができるという意味ではありません。

`NEAR_SANDBOX_BIN_PATH` に実在する sandbox 2.11.0 の絶対パスを指定します。契約統合テストでは指定しないと依存 crate が実行時ダウンロードを試みることがあり、Cargo の `--offline` だけでは子プロセスの通信を止められません。

## ソケットの権限がない場合

```text
Local sandbox could not start: Error while binding socket to a port 0:
Operation not permitted (os error 1)
```

この場合は localhost の bind / listen を許す通常シェルで、同じコード・資材を使って再実行します。権限変更や制限回避は行いません。testnet アカウント・秘密鍵は不要です。デモは外部 peer / telemetry / RPC フォールバックを無効化しています。

```sh
scripts/demo.sh
```

毎回のログは `demo-artifacts/demo-output.txt`、再検証用 JSON は `demo-artifacts/path/*.json` / `demo-artifacts/spend/*.json` です。既存の公開参照ログを上書きしません。両題材各5版、計10版の `matched` とプロセスの exit 0 を両方確認してください。

## 継続する未確認事項

- 持込み wasm とソースの同一性。R1 では wasm 標準ライブラリがなく再ビルドできていません。
- `wasm32-wasip2` 標準ライブラリ不足による judge の WASI ビルド・ランタイム実行。ネイティブの Lean 子プロセスは WASI 対応していません。
- 本物の TEE / testnet。local の attestation は mock です。
- 公開用に匿名化したテスト wasm のチェーン実行。既存 FIX5 ホストログは開発コピーの資材による記録で、公開コピーの実行確認とは区別します。
- 契約 clippy の診断は R2 で解消済み。T005 の all-targets / -D warnings も成功し、依存 wasmparser の future-incompatibility 通知のみ継続します。
- 第三者ライセンス一覧の不足警告と、judge / Lean を含む現行 SBOM。ローカルに不足ライセンス原文がなく、外部取得もしていません。

## 2026-11-10 運用者ホスト確認 (path-5)
R3 修正後、サンドボックス外で lake build・judge/契約の全テスト・scripts/demo.sh を実行し、すべて成功(demo exit 0、5版とも chain/verify matched)。ログ: docs/demo-output-host-path5.txt

## 2026-11-10 運用者ホスト確認 (path-6, T005 後)
lake build・judge 47件・契約 72件・demo.sh すべて成功(exit 0、5版とも matched)。ログ: docs/demo-output-host-path6.txt

## 2026-11-10 運用者ホスト確認 (T006 後, path-7 / spend-1)
lake build 両題材・答え漏れ検査・judge 57件・契約 72件・demo.sh 両題材すべて成功(exit 0、10版とも matched)。ログ: docs/demo-output-host-t006.txt

## 2026-11-10 運用者ホスト確認 (FIX5 後)
漏れ検査から内部 QA 報告 docs/qa/ を除外(公開対象外・ブラインドコピー対象外)。両 lake build・demo.sh 両題材10版 matched / exit 0 は[デモログ](demo-output-host-fix5.txt)で確認。judge66件・契約72件の成功は運用者の実行報告（このログに全テスト明細はない）。
