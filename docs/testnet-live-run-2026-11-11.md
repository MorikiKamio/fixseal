# Live AI fix + NEAR testnet guard run — 2026-11-11

**Simulated environment, synthetic data**: disposable testnet accounts, local/mock attestation, no real TEE and no real funds. It is not a security certification.

Two live runs were made on 2026-11-11, each on a fresh contract. **Run B (`fixseal-live-3.testnet`) is the one shown in the demo video** ([jump](#run-b-fixseal-live-3testnet--the-demo-video)). Run A (`fixseal-live-2.testnet`) was the first take. In both, Codex was accepted on the first attempt and the guard installed exactly its rule.

## Run A: `fixseal-live-2.testnet` (first take)

What is different from the [2026-11-10 run](testnet-run-2026-11-10.md): the accepted fix was not prepared in advance. Codex wrote it during the recording in an answer-withheld copy (`scripts/live-fix.sh`), the judge accepted it on the first attempt, and the guard then installed exactly that rule.

| Item | Value |
| --- | --- |
| Run (UTC) | 2026-11-11T01:24Z |
| Network | testnet |
| Contract | [`fixseal-live-2.testnet`](https://testnet.nearblocks.io/address/fixseal-live-2.testnet) |
| Live viewer | <https://morikikamio.github.io/fixseal/?contract=fixseal-live-2.testnet> |
| Agent / recipient | `fixseal-agent.testnet` / `fixseal-recipient.testnet` |
| Agent spend key | `ed25519:3ERDv5Do5kEtiEdTwgqNkxBpKvym477ewsFzdtk9mX5v`, function-call only: receiver `fixseal-live-2.testnet`, method `guard_spend` |
| Deployed wasm SHA-256 | `9fb005949b916925053a0d0781b193e7f4ab74aebcd6761eb730145de9f4c718` (same as the GitHub CI source build; the viewer rechecks it) |
| Judge | `0.1.0/lean-spend-4` |
| Fixed specification SHA-256 | `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe` |

## 1. AI fixer (Codex, live)

| Item | Value |
| --- | --- |
| Fixer | Codex (`gpt-6.1-sol`), `codex exec -s workspace-write`, blind copy with the answer and archived fixes removed |
| Attempts | 1 (accepted on the first attempt, about 3 minutes) |
| Behaviour | attacks working 12/12 → 0/12, regressions 16/16 |
| Proof | Lean kernel check passed against the fixed specification |
| Codex's rule | `if spent ≤ limit then amount ≤ limit − spent else false` |
| Policy expression SHA-256 | `2dd8b636fd4ebe9cf17a14182636d54daa693f13bef703a7a32321f84fe6df76` |
| Accept record ID (on chain) | `a52b5ab3c83c4a329be3aaa93c31bca6801a372622247c89f3bcb8eae9f70535` |

Codex's [Target.lean](../examples/fixers/spend/codex-live-2026-11-11/Target.lean), [Proof.lean](../examples/fixers/spend/codex-live-2026-11-11/Proof.lean) and the [accepted evidence bundle](../examples/fixers/spend/codex-live-2026-11-11/accepted.json) are public. Recheck the bundle offline:

```sh
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify examples/fixers/spend/codex-live-2026-11-11/accepted.json
```

Expected: `VERIFIED` (re-export, replayed report and result hash). The fixer's raw log is not published because it contains local paths.

A single first-attempt accept is one run, not a model ranking. In the earlier recorded runs Devin and Codex passed every test on the first try and were still rejected for wrong proofs ([records](../README.md#real-ai-fixers-blind-run)).

## 2. Guard run (`scripts/testnet-guard.sh --execute --accepted-bundle`)

Each outcome was checked from final receipts by the script, not only from the CLI exit code.

| # | Step | Outcome | Transaction |
| --- | --- | --- | --- |
| 1 | Initialize the vulnerable policy (limit 1000 yoctoNEAR) | success | [`8rbJ8x6G…`](https://testnet.nearblocks.io/txns/8rbJ8x6GKrmjkTVuqQvQuZHjm2cT4UdRxW5i4rypQtEr) |
| 2 | Spend 600 | success | [`tUr8SkFW…`](https://testnet.nearblocks.io/txns/tUr8SkFWV1Y8zDhXiXjMimZ5Cqcn88JjVzRfdszz5YG) |
| 3 | Overflow attack: spend 18446744073709551116 | **paid** (the bug) | [`CmRfwBZy…`](https://testnet.nearblocks.io/txns/CmRfwBZyKGAMjJNThXYjDJkSncsXQcfoVgw2LTHZikAh) |
| 4 | Replace policy without a record | rejected (expected) | [`8PN28d7t…`](https://testnet.nearblocks.io/txns/8PN28d7teQXrJ9HBrPtUmmXDygw8E3Zrse9JKQm9xfgn) |
| 5 | Record the saturating fix (judge verdict: reject) | recorded | [`6PEP8iQS…`](https://testnet.nearblocks.io/txns/6PEP8iQSGTo9KjKE8fVnXTu9Mw435oWsodZPDHBLaBKL) |
| 6 | Install the saturating fix | rejected (expected) | [`G7D9gJwC…`](https://testnet.nearblocks.io/txns/G7D9gJwCn9V81iPdzu8sXfkkG29ChtBZJuL5y1UeWUnS) |
| 7 | Record **Codex's live fix** (judge verdict: accept, Lean proof checked) | recorded | [`6bxdneTT…`](https://testnet.nearblocks.io/txns/6bxdneTTNfYwijR8rzQGhriqgdXmJm89qLh3briDnQoc) |
| 8 | Install Codex's fix | success | [`BzaxTsaY…`](https://testnet.nearblocks.io/txns/BzaxTsaYWpfaLgfxSKRfjsj5bPKn2P7j826LUJ9yja2j) |
| 9 | Replay the same overflow attack | rejected (expected) | [`Htf4yuSY…`](https://testnet.nearblocks.io/txns/Htf4yuSYkdSWNFrQu6gmgpetWdF8kyRzeybtnJyp8vhH) |
| 10 | Legitimate spend 100 | success | [`4rwZeU8f…`](https://testnet.nearblocks.io/txns/4rwZeU8f7zZ8h4U5ZZAEp2ZVqJGJxWqTN9mL71MCZjUP) |

Final `guard_state`: `policy_expr` is Codex's rule above, `record_id` `a52b5ab3…0535`, `spent` 200 (wrapped by the step-3 overflow), `total_paid` 18446744073709551816 yoctoNEAR, `pending_paid` 0. The script compared the recipient's on-chain balance with these payments and ended with “final guard state and recipient balance checks passed”.


## Run B: `fixseal-live-3.testnet` — the demo video

Same contract code, judge, specification and 10 steps as run A, on a fresh contract. The video shows this run end to end (fixer phase at 5× speed, guard phase at 2×; the Codex output panel is replayed from this run's log).

| Item | Value |
| --- | --- |
| Guard run (UTC) | 2026-11-11T03:01Z |
| Contract | [`fixseal-live-3.testnet`](https://testnet.nearblocks.io/address/fixseal-live-3.testnet) |
| Live viewer | <https://morikikamio.github.io/fixseal/?contract=fixseal-live-3.testnet> |
| Agent spend key | `ed25519:6v1fyE3eMu6AuyfJAmjZm9EwFfBwHYTTRHmHfynRJ9Tn`, function-call only: receiver `fixseal-live-3.testnet`, method `guard_spend` |
| Fixer | Codex (`gpt-6.1-sol`), fresh blind copy, 1 attempt, accepted |
| Behaviour | attacks working 12/12 → 0/12, regressions 16/16, Lean proof checked |
| Codex's rule | `if spent ≤ limit then amount ≤ limit − spent else false` (same expression as run A; the proof text differs) |
| Policy expression SHA-256 | `2dd8b636fd4ebe9cf17a14182636d54daa693f13bef703a7a32321f84fe6df76` |
| Accept record ID (on chain) | `4aca5779a49abb49f632421ee8371211ce3bef3f2ca0dbbca01a4469b2d9d78b` |

Codex's [Target.lean](../examples/fixers/spend/codex-live-2026-11-11-video/Target.lean), [Proof.lean](../examples/fixers/spend/codex-live-2026-11-11-video/Proof.lean) and [accepted bundle](../examples/fixers/spend/codex-live-2026-11-11-video/accepted.json) (`fixseal-judge verify` → `VERIFIED`).

| # | Step | Outcome | Transaction |
| --- | --- | --- | --- |
| 1 | Initialize the vulnerable policy (limit 1000 yoctoNEAR) | success | [`GCehcnYi…`](https://testnet.nearblocks.io/txns/GCehcnYi32vzWq4mm2fNgDXdc14j21yZsxeAiwTm4DL1) |
| 2 | Spend 600 | success | [`9nhBQw8h…`](https://testnet.nearblocks.io/txns/9nhBQw8hnXAY8R91PDT1qTfmoG9r4F1VjNZwbGyb1TCF) |
| 3 | Overflow attack: spend 18446744073709551116 | **paid** (the bug) | [`4v2YK4n5…`](https://testnet.nearblocks.io/txns/4v2YK4n5JwkLCxdyGQAp2dfJx27Y37Qp1zTTPVmQUsm8) |
| 4 | Replace policy without a record | rejected (expected) | [`GN2cd1mD…`](https://testnet.nearblocks.io/txns/GN2cd1mDKbwABwH62W8t4zRgc2LfV4hTdHKxREJhtkxx) |
| 5 | Record the saturating fix (judge verdict: reject) | recorded | [`BZe8RFQV…`](https://testnet.nearblocks.io/txns/BZe8RFQVDSS6o48NVn3Gdh3TmfHj9nFqC7frDaqm2y2k) |
| 6 | Install the saturating fix | rejected (expected) | [`CZnRLAwa…`](https://testnet.nearblocks.io/txns/CZnRLAwaXAfPmJuUHHY9cahc4pfFEmr4vXyrr36VXhqU) |
| 7 | Record **Codex's live fix** (judge verdict: accept, Lean proof checked) | recorded | [`DDcoSUc4…`](https://testnet.nearblocks.io/txns/DDcoSUc4N9kD4bsFp4bT1Es1dzBgmrnmmvfy2BB2WXk5) |
| 8 | Install Codex's fix | success | [`CLozULM9…`](https://testnet.nearblocks.io/txns/CLozULM9yKkEUk31CN18LVhP7E6p8iKmaxABFboN5q7S) |
| 9 | Replay the same overflow attack | rejected (expected) | [`F9nHgtWK…`](https://testnet.nearblocks.io/txns/F9nHgtWKhzd9yGQn3gGbPymHGEjPGD1J3jASGggjJXvN) |
| 10 | Legitimate spend 100 | success | [`92mVWGPE…`](https://testnet.nearblocks.io/txns/92mVWGPEBpkxehzM9mdhMDePiFXZ6XMEbJPZeeh73qJA) |

Final `guard_state`: Codex's rule, `record_id` `4aca5779…d78b`, `spent` 200 (wrapped by the step-3 overflow), `total_paid` 18446744073709551816 yoctoNEAR, `pending_paid` 0. The script ended with "final guard state and recipient balance checks passed".

## Limits

Same as the [2026-11-10 run](testnet-run-2026-11-10.md#limits): the contract trusts the record submitter and the owner and does not run Lean; attestation is mock; the Lean-to-contract correspondence is checked on 1,028 fixed inputs, not proved.
