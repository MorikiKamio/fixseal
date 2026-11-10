# Setup and offline reproduction

A clone contains source and lockfiles, not toolchains, Cargo caches, sandbox executables or contract wasm. Use Rust/Cargo 1.99.1, Lean/Lake 4.34.1, a native C compiler and Python 3. Both lockfiles need matching cached dependencies. No external Lean packages are needed. Node, Docker and a testnet account are unnecessary for the prepared offline route.

Point `FIXSEAL_LEAN_BIN` at the installed native Lean 4.34.1 `bin` directory and add it to PATH for direct Lake commands. The judge uses native binaries rather than an auto-downloading proxy. From the repository root:

```sh
export FIXSEAL_LEAN_BIN="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
export PATH="$FIXSEAL_LEAN_BIN:$PATH"
(cd lean && lake build && lake env lean SpecTests.lean)
(cd lean/targets/spend && lake build)
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests
```

The separate chain route needs preinstalled near-sandbox 2.11.0, contract caches, `agent-contract/target/near/shade_contract_template.wasm`, and permission to bind/listen on localhost. Set `NEAR_SANDBOX_BIN_PATH` to an existing absolute executable path to prevent runtime downloads. `scripts/demo.sh --check` checks presence only; `scripts/demo.sh` performs both subjects' deploy/register/submit/readback/replay sequence. It requires explicit loopback permission. Socket permission errors are environment failures, not successful tests. The [T008 operator host record](demo-output-host-t008.txt) covers path-7/spend-2 (ten matched, exit 0); FIX5 spend-3 awaits operator reconfirmation. All figures describe a **simulated environment, synthetic data**.

With the matching Rust wasm target already installed, rebuild the contract offline using:

```sh
cargo build --manifest-path agent-contract/Cargo.toml --target-dir agent-contract/target --target wasm32-unknown-unknown --release --locked --offline
mkdir -p agent-contract/target/near
cp agent-contract/target/wasm32-unknown-unknown/release/shade_contract_template.wasm agent-contract/target/near/shade_contract_template.wasm
```

Initial preparation requires separately authorized downloads on a capable machine: Rust 1.99.1 and its wasm target, Lean 4.34.1, locked Cargo dependencies and the sandbox for the target OS/CPU. Transfer prepared caches/binaries into restricted environments. The historical installation examples below were not executed here; do not run them when network access or installation is prohibited. Supplied wasm/source correspondence and a cold setup remain unconfirmed. See [JUDGES](../JUDGES.md) for expected outputs and troubleshooting.

## Japanese reference and historical preparation commands

# 初回準備とオフライン再現

クローンには toolchain、依存キャッシュ、NEAR sandbox バイナリ、ビルド済み契約を含めません。この文書は初回準備と、準備済み環境での実行を分けています。R1 では外部取得・インストールを実施しておらず、以下の初回導入と契約の再ビルドは未実行です。

## バージョンと必要物

| 資材 | 対応・確認した版 | 必要な経路 |
| --- | --- | --- |
| Rust / Cargo | 1.99.1（既存環境）。edition 2024 対応が必要 | judge / 契約テスト |
| Lean / Lake | `leanprover/lean4:v4.34.1` 固定、外部 Lean パッケージなし | Lean judge |
| C コンパイラ | 導入済みのネイティブコンパイラ | Lean 実行ファイル |
| Cargo 依存 | 各 `Cargo.lock` 固定。near-sdk 5.26.1 / near-api 0.8.6 / near-sandbox crate 0.3.9 | 各 crate |
| NEAR sandbox 実行ファイル | 2.11.0 | チェーンデモ・統合テスト |
| Rust wasm 標準ライブラリ | 使用する Rust 用 `wasm32-unknown-unknown` | 契約の初回ビルド |
| 契約 wasm | `agent-contract/target/near/shade_contract_template.wasm` | チェーンデモ・統合テスト |
| Python | 3（標準ライブラリのみ） | 作業コピーからの公開書き出し・補助テスト |

macOS arm64 とホストの既存資材での記録があります。他 OS は未検証です。Node、Docker、Shade CLI、testnet / Phala のアカウントは Fixseal のローカル検証には不要です。

## 初回準備（外部取得が許可された環境だけ）

既存の Rust、C コンパイラ、elan を用意した環境で行う例です。通信禁止の作業中には実行しません。オフライン環境へは、同じ OS / CPU / toolchain の環境から toolchain、Cargo キャッシュ、sandbox、wasm を別途持ち込んでください。

```sh
rustup toolchain install 1.99.1
rustup target add wasm32-unknown-unknown --toolchain 1.99.1
elan toolchain install leanprover/lean4:v4.34.1
cargo +1.99.1 fetch --manifest-path judge/Cargo.toml --locked
cargo +1.99.1 fetch --manifest-path agent-contract/Cargo.toml --locked
```

NEAR sandbox は `near-sandbox` crate 0.3.9 が指定する 2.11.0 を用意します。ローカルの同 crate の `src/runner/mod.rs` にある配布先は次の形式です（このセッションでアクセスしていません）。

```sh
# 対応する値を選ぶ: Darwin-arm64 / Linux-x86_64 / Linux-aarch64
FIXSEAL_PLATFORM=Darwin-arm64
mkdir -p "$HOME/.near/near-sandbox-2.11.0"
curl --fail --location "https://s3-us-west-1.amazonaws.com/build.nearprotocol.com/nearcore/$FIXSEAL_PLATFORM/2.11.0/near-sandbox.tar.gz" -o /tmp/fixseal-near-sandbox.tar.gz
tar -xzf /tmp/fixseal-near-sandbox.tar.gz -C "$HOME/.near/near-sandbox-2.11.0"
export NEAR_SANDBOX_BIN_PATH="$HOME/.near/near-sandbox-2.11.0/near-sandbox"
"$NEAR_SANDBOX_BIN_PATH" --version
```

この URL / 対応プラットフォームはローカルの依存ソースによるもので、配布先の現状・ダウンロード内容の真正性は未検証です。信頼できる配布元の資材を使ってください。macOS x86_64 は同 crate の配布対象外です。

## 契約 wasm の生成（準備後はオフライン）

リポジトリ直下で、wasm 標準ライブラリと locked 依存キャッシュを用意した Rust を使います。`cargo` / `rustc` が同じ toolchain を指すよう PATH を設定します。

```sh
cargo build --manifest-path agent-contract/Cargo.toml --target-dir agent-contract/target --target wasm32-unknown-unknown --release --locked --offline
mkdir -p agent-contract/target/near
cp agent-contract/target/wasm32-unknown-unknown/release/shade_contract_template.wasm agent-contract/target/near/shade_contract_template.wasm
```

契約の metadata に記録された別の生成手順は、cargo-near 0.19.0 を用意した上で `(cd agent-contract && CARGO_NET_OFFLINE=true cargo near build non-reproducible-wasm --locked --no-abi)` です。R1 の環境では wasm 標準ライブラリがなく、どちらの再ビルドも未検証です。持込み wasm と現在のソースの同一性を確認したとは主張しません。

## 準備済み環境での検証（外部取得なし）

`cargo` / `rustc` を PATH に置きます。Lean を elan 以外の場所に導入した場合、`FIXSEAL_LEAN_BIN` に Lean 4.34.1 の実体の bin ディレクトリを指定します。judge はこの信頼する toolchain を直接使い、自動ダウンロードする proxy にフォールバックしません。

```sh
export FIXSEAL_LEAN_BIN="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
export PATH="$FIXSEAL_LEAN_BIN:$PATH"
(cd lean && lake build && lake env lean SpecTests.lean)
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --lib
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test local_demo
```

最後のコマンドは summary 形式のテストを行い、チェーンデモ本体は opt-in です。ここまでは NEAR sandbox と wasm、localhost bind 権限が不要です。

全契約統合テストとチェーンデモは、追加資材と localhost の bind / listen 権限が必要です。外部通信禁止時は、Cargo の `--offline` に加え、実在するバイナリを必ず明示してください。指定しなければ `near-sandbox` crate が実行時ダウンロードを試みることがあります。

```sh
export NEAR_SANDBOX_BIN_PATH="$HOME/.near/near-sandbox-2.11.0/near-sandbox"
scripts/demo.sh --check
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --no-fail-fast
scripts/demo.sh
```

`--check` は存在確認のみです。ソケット権限による停止をテスト成功として扱いません。実行ログは `demo-artifacts/demo-output.txt`、JSON は `demo-artifacts/*.json` に保存します。[path-5 のホスト成功記録](demo-output-host-path5.txt)と、[制限環境での状態](BLOCKED.md)を区別してください。
