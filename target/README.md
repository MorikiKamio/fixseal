# Synthetic path target: English summary

This is the historical VM subject, a **simulated environment, synthetic data**, with no actual file I/O. Each `path.fn` is interpreted by the [judge](../judge/README.md): `before` decodes/joins/normalizes and can escape the root; `after_honest` checks containment; `after_hardcode` blocks only the public attack; `after_weaken_test` rejects everything and removes normal regressions. The judge always reruns baseline regressions and rejects test modifications. Use `cargo run --manifest-path judge/Cargo.toml --locked --offline -- target after_honest`. The primary kernel-checked subjects live under `lean`.

## Japanese reference

# ダミーのパス結合ターゲット

合成データによる「仮想空間での検証」用。各 `path.fn` は [judge の命令言語](../judge/README.md) で書いたパス結合関数で、実ファイルを開かない。

| 版 | 動作 |
| --- | --- |
| `before` | URL デコード・結合・正規化のみ。許可ルート外も返す。 |
| `after_honest` | 正規化後に許可ルート内か確認する。 |
| `after_hardcode` | 公開攻撃文字列だけ拒否する。 |
| `after_weaken_test` | 全入力を拒否し、正常系の回帰テストを取り除く。 |

`before/tests/regression.json` が判定に使う元の回帰テスト。他の版も同じファイルを持ち、`after_weaken_test` だけが改変している。`corpus.json` と `policy.json` は全版に共通の信頼入力として judge に渡す。攻撃コーパスのパスは架空で、実データは含まない。
