# Devin (swe-2-high): blind spend repair

## English summary

**simulated environment, synthetic data**. In the recorded blind run, the agent worked in a copy without the reference answer, with UInt64/Bool-only Target arithmetic and no network access. Attempt 1 passed all behavioral checks (attacks 0/12, regressions 16/16) but was rejected with `PROOF_REJECTED` because of an invalid rewrite argument. After reading Lean's error, attempt 2 was accepted; the kernel checked the proof against the fixed specification and the fixed tests were rerun. The final bundle replay reported `VERIFIED`; no chain run was performed.

The [archived attempts](../../examples/fixers/README.md) are post-experiment records. Attempt 2 is transcribed from the final source below. Attempt 1's complete original source was not retained in this record: its Proof is reconstructed from the final source, the described edit and the quoted diagnostic. It is not a byte-identical historical artifact. T008 locally rechecks both attempts with judge `0.1.0/lean-spend-2`; this is a replay, not a new blind run.

# FX3 記録 (担当: Devin)

合成データによる**仮想空間での検証**の記録。

## 最終結果

- verdict: `accept`、理由コード: `[]`
- 攻撃: baseline 12/12 成功(脆弱性再現)、candidate 0/12 成功
- 回帰: 16/16 通過
- 証明: verified `true`(公理依存は `propext`, `Classical.choice`, `Quot.sound` のみ)
- 判定コマンド: `cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean --subject spend lean after_fx3`
- bundle 出力→`verify` で再現検証済み: `VERIFIED`(demo-artifacts/after_fx3.json)

## 試行履歴

判定 2 回。

1 回目: `PROOF_REJECTED`。攻撃・回帰はすべて通過(0/12・16/16)したが、証明のビルドで失敗。Lean のエラー文:

```
error: after_honest/Proof.lean:6:6: Invalid rewrite argument: Expected an equality or iff proof or definition name, but `of_decide_eq_true
  ?a✝` is a proof of
  ?m.9
```

原因: `of_decide_eq_true` はこの toolchain では `Decidable p → (decide p = true) → p` という定理であり、rw に使える iff ではなかった。`decide p = true ↔ p` の形を持つ `decide_eq_true_iff`(Init/PropLemmas.lean)に書き換えて修正。

2 回目: `accept`。

## 修正内容

脆弱性: `before/` は `decide (spent + amount ≤ limit)` で、UInt64 の加算は 2^64 で wrap するため `spent + amount` が桁あふれで小さく見え、上限超過を許可していた。

修正: 加算そのものを避け、`spent ≤ limit` を確認した上で `amount ≤ limit - spent` を見る形に変更。`spent ≤ limit` のとき `limit - spent` は数学的な差と一致し、`amount ≤ limit - spent` は `spent + amount ≤ limit` と同値。`spent > limit` では第一項が偽になり必ず拒否する。Target は UInt64 と Bool の演算(`decide`, `≤`, `∧`, `-`)のみで構成。

## 最終ファイル全文

`lean/targets/spend/after_fx3/Target.lean`:

```lean
import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  decide (spent ≤ limit ∧ amount ≤ limit - spent)

end Target
```

`lean/targets/spend/after_fx3/Proof.lean`:

```lean
import Target

theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  unfold Target.authorize
  rw [decide_eq_true_iff, UInt64.le_iff_toNat_le, UInt64.le_iff_toNat_le]
  by_cases h : s.toNat ≤ l.toNat
  · rw [UInt64.toNat_sub_of_le l s (UInt64.le_iff_toNat_le.mpr h)]
    omega
  · omega
```

証明の概略: `decide_eq_true_iff` で `decide _ = true` を命題に戻し、`UInt64.le_iff_toNat_le` で UInt64 の `≤` を `toNat` の比較に変換する。`s.toNat ≤ l.toNat` で場合分けし、真のときは `UInt64.toNat_sub_of_le` で `(l - s).toNat = l.toNat - s.toNat` に書き換えて `omega`、偽のときは両辺が偽なので `omega` で閉じる。

## 掛かった時間

約 25 分(読み込み・実装・判定 2 回・記録を含む)。

## 環境上の未解決事項

`python3 scripts/check-spend-proof-leak.py --self-test` は、この開発コピーに `lean/targets/spend/after_honest/` が存在しないため `FileNotFoundError` で実行できなかった(環境由来の失敗。自分の証明文の漏洩チェックではなく、参照証明の不在によるもの)。新規ファイル `after_fx3/Proof.lean` は `after_honest` の内容を参照せず独自に記述した。

外部通信・パッケージインストール・commit・push は行っていない。正解版(`after_honest`)はこのコピーに存在せず、他のディレクトリ・ネットワークも参照していない。

## T008: 公開用アーカイブと復元

初回 Proof は最終版の最初の書換え引数を `of_decide_eq_true` に戻して復元した。 初回 Target は記録の最終 Target と同じ。初回ソースの完全な同一性は確認できないが、現行 judge で初回 `PROOF_REJECTED`、2回目 `accept`、両回とも攻撃0/12・回帰16/16を確認するテストを追加した。最終 Proof は import・空行を含めて10行。独立した修正担当へ渡すコピーからは、参照解と本記録・アーカイブを除く。
