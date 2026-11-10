# fixers/nearai

NEAR AI Cloud のモデルを Fixseal の修正担当として走らせる小さな道具。
「NEAR AI Cloud のモデルが修正と Lean の証明を書き、Fixseal が判定した」
記録を足すためのもの。Python 3 標準ライブラリのみ。実際の実行（API キー
と別途の外部通信許可が要る）は運用者が行う。開発コピー・公開exportを
そのままworkdirにしない。参照解答を含むコピーは実行前に拒否する。

## 使い方（3コマンド）

```sh
# 1. 開発コピーから答えを抜いたコピーを作る（漏れ検査つき）
fixers/nearai/make_blind_copy.sh /path/to/blind-copy

# 2. API キーを環境変数に入れて実行（リポジトリ直下から）
export NEARAI_API_KEY=sk-...
python3 fixers/nearai/run_fixer.py \
    --workdir /path/to/blind-copy \
    --instruction docs/tasks/<修正担当への指示ファイル>.md \
    --subject spend --version after_nearai \
    --out-dir demo-artifacts/nearai

# 3. 結果を見る
cat demo-artifacts/nearai/judgements.jsonl   # 各回の判定
cat demo-artifacts/nearai/record.md          # docs/fixer/ 用の記録
```

- `--instruction`: モデルに渡す指示文のパス。`docs/tasks/` の FX3 と同じ
  内容のファイルを運用者が用意して渡す。`--version` は指示文にある候補名
  と一致させること（モデルがそのディレクトリをコピーして作る）。
- 前提: コピー内で judge が動くこと。Rust/Cargo、Lean 4.34.1、依存
  キャッシュ、必要なら `FIXSEAL_LEAN_BIN` 環境変数。判定コマンドは
  `cargo run --manifest-path judge/Cargo.toml --locked --offline --`
  `--lean [--subject spend] lean VERSION`（コピー内で実行）。
- モデルが使える道具は `bash` と `done` のみ。`bash` は作業ディレクトリ
  内でのみ実行され、ネットワークを使うコマンド（curl、wget、git clone、
  pip、npm、lake update など）は実行前に拒否される。作業ディレクトリ外の
  パス・ホーム相対パス・パス内の変数展開も拒否する。1 コマンドの時間制限
  （既定 600 秒）と出力の切り詰めあり。
- `done` が呼ばれるたびに judge を 1 回実行する。accept でなければ
  verdict・理由・抽出した診断をモデルに返して続ける。bashでacceptを
  見た後、またはdocs/fixer/への記録書き込み後は、次のターンにdoneを
  呼ぶよう明示して促す（出力/コマンドに基づく補助検出）。道具を呼ばずに
  止まったら 1 行で促す。`--max-turns` / `--max-seconds` /
  `--max-tokens`（累計）で停止。
- done未呼び出しのままターン上限に達した場合、道具がjudgeを1回だけ
  実行して`final_judge`として記録する。`done_called`と`attempts`
  （done経由のみ）は別に記録し、max_turnsはacceptでもそのまま残す。
- accept になったら bundle 出力と `verify` まで自動で行い
  （`--no-bundle-verify` で省略可）、VERIFIED かを記録する。
- `NEARAI_API_KEY` はログ・記録・例外文・子プロセスの環境から除外される。

## 主な引数

| 引数 | 既定 | 説明 |
| --- | --- | --- |
| `--base-url` | `https://cloud-api.near.ai/v1`（`NEARAI_BASE_URL`） | docs.near.ai で確認した gateway |
| `--model` | `zai-org/GLM-5.1-FP8`（`NEARAI_MODEL`） | quickstart の例。変更可 |
| `--subject` | `spend` | `path` か `spend` |
| `--max-turns` | 40 | API 呼び出し回数の上限 |
| `--max-seconds` | 14400 | 全体の壁時計上限 |
| `--max-tokens` | 4000000 | 累計 usage の上限 |
| `--command-timeout` | 600 | bash 1 コマンドの秒数 |
| `--judge-timeout` | 1200 | 判定 1 回の秒数 |

## 費用の目安の出し方

実行開始時に、同じbase URLの`/models`（既定は`/v1/models`）を取得し、
選択モデルのpricing・取得時刻・出典を保存する。usageを累計し、終了時に
`cost`としてUSD概算と計算に使った単価/usageをJSON Linesと記録へ書く。
API未実行のNX1履歴には単価スナップショットがなく、遡って費用を作らない。

対応するpricing形式:

- `prompt` / `completion`（USD/token）、任意の`input_cache_read`。
- `input` / `output`と明示した`unit`（`usd_per_token` または
  `usd_per_million_tokens`）、任意の`cache_read`。

`prompt_tokens_details.cached_tokens`が全応答にあり、キャッシュ単価も
ある場合だけ割引する。内訳がなければ通常入力単価で`upper_bound`
（キャッシュ割引前の上限）とする。料金は請求実績ではない。単価取得失敗、
未対応の形式/単位、usage欠落は`unavailable`として記録し、0ドルと扱わない。
今回の実装テストはAPIを呼ばず、実サービスのpricing形式は未確認。

## 限界

- `bash` の拒否リストは協力的なエージェント向けのガードであり、完全な
  サンドボックスではない。難読化されたネットワークアクセスやシェル
  展開による脱出は防げない場合がある。実行環境の選択は運用者の責任。
- 判定 JSON に Lean の診断は含まれないため、記録のエラー要約は judge
  の stderr に現れた `error` 行に限る。必要ならモデル自身が直接
  コンパイルして診断を読む（FX3 と同じやり方）。
- `record.md` は自動生成。項目は FX3 の記録と同じだが「修正内容」は
  モデルの自己報告のみで、公開前に運用者の確認が必要。
- 非ストリーミングの chat completions のみ実装。`reasoning_content`
  は記録に使わない。Responses API・モデル個別エンドポイントは未対応。
- 時間/トークン/APIエラーによる停止は自動再判定しない。ターン上限時の
  `final_judge`と`done_called`を区別し、`stop_reason`も確認すること。
- 元のNX1実行後に、bundleのパス組み立てとverifyログのclose順も修正。
  verify失敗は記録し、終了コードも失敗とする。

## テスト

```sh
python3 fixers/nearai/test_run_fixer.py
```

標準ライブラリのin-process偽クライアント・HTTPレスポンスの差し替えを使う。
API通信・localhost bindは行わない。judge fallback、催促、費用の割引/上限、
キー非漏洩、bundle/verify記録の一連の流れを確認する。
