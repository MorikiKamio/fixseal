# NEAR testnet guard run — 2026-11-10

**Simulated environment, synthetic data** (仮想空間での検証): disposable testnet accounts, local/mock attestation (`requires_tee: false`), no real TEE and no real funds. This shows that the guard's rules hold on a public NEAR network; it is not a security certification.

| Item | Value |
| --- | --- |
| Run (UTC) | 2026-11-10T11:53:59Z |
| Network | testnet |
| Contract | [`fixseal-guard.testnet`](https://testnet.nearblocks.io/address/fixseal-guard.testnet) |
| Owner / agent / recipient | `fixseal-owner.testnet` / `fixseal-agent.testnet` / `fixseal-recipient.testnet` |
| Deployed wasm SHA-256 | `9fb005949b916925053a0d0781b193e7f4ab74aebcd6761eb730145de9f4c718` (same as the GitHub CI source build) |
| Judge | `0.1.0/lean-spend-4` |
| Fixed specification SHA-256 | `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe` |
| Accepted record ID | `c3648c7670ef2095eed61998a699c09d4bc2f8054a81df03272fcdc406d9ffcb` |
| Installed policy SHA-256 | `f23ed0cdfa62e12466cc568310c6880484f321a946dbeeb0c28f99fae8db330a` |
| Agent spend key | `ed25519:7EdXbephz5fZrYvBbr6t5mwoPNUZxik1L3fGPNT1PCvQ`, function-call only: receiver `fixseal-guard.testnet`, method `guard_spend` |

## Setup (operator)

| Step | Transaction |
| --- | --- |
| Deploy wasm + `new` (owner, local/mock mode) | [`Gh3aziKh…`](https://testnet.nearblocks.io/txns/Gh3aziKhH6oUcEtGNawJsu7sJts5TgSnF7s82ukSTEx3) |
| Approve default mock measurements | [`6G6V4Dxw…`](https://testnet.nearblocks.io/txns/6G6V4Dxwpu1UfGMfN1GpRL49KHvnzXhHYPaoBzJ3dTSJ) |
| Approve default mock PPID | [`A2toxaT6…`](https://testnet.nearblocks.io/txns/A2toxaT6DeoVZS8VvQhtwoCeo3tp2beJMQwuGZuYhANJ) |
| Whitelist agent for local mode | [`jyCJXEaq…`](https://testnet.nearblocks.io/txns/jyCJXEaq4g4fWcKupAerrTx2DG8ApB6EqLEgMPLxiM9) |
| Register agent (mock attestation) | [`A7JFB1ra…`](https://testnet.nearblocks.io/txns/A7JFB1razEqUa7RXq5pLuL1WNHnFTb7k5tyAij4gxw8J) |
| Add the restricted spend key | [`FmjAByYZ…`](https://testnet.nearblocks.io/txns/FmjAByYZBPNkLjT4RQNaLTw1WFQbBNzkJEX1qq2BtxiQ) |

## Guard run (`scripts/testnet-guard.sh --execute`)

Each outcome below was checked from final receipts by the script, not only from the CLI exit code.

| # | Step | Outcome | Transaction |
| --- | --- | --- | --- |
| 1 | Initialize the vulnerable policy (limit 1000 yoctoNEAR) | success | [`DCcJEiae…`](https://testnet.nearblocks.io/txns/DCcJEiae8DFcGKXvxXdpjHNu9MJ9N56a9HzxhWo9xR2g) |
| 2 | Spend 600 | success | [`4TU54yHn…`](https://testnet.nearblocks.io/txns/4TU54yHnkaJ3sp9g9hn7Kd9TjDvjKJ3aBPZnJWmbNjcN) |
| 3 | Overflow attack: spend 18446744073709551116 | **paid** (the bug) | [`3Y7v93Br…`](https://testnet.nearblocks.io/txns/3Y7v93BrSSSUfVeJjswo6DxW5E6gW7R8ka4FUC3Vc27L) |
| 4 | Replace policy without a record | rejected (`Guard record missing`) | [`5XCqC9F1…`](https://testnet.nearblocks.io/txns/5XCqC9F1uBPcmXVLMFh4RwdsSASFjXuNYkU8pET7CyP2) |
| 5 | Record the saturating fix (judge verdict: reject) | recorded | [`BQRrn5YK…`](https://testnet.nearblocks.io/txns/BQRrn5YKTXWUKKVdhZ5zTm41F9Sg2uRSPumoWtjnGqTr) |
| 6 | Install the saturating fix | rejected (`Guard record is not accept`) | [`CU4UED48…`](https://testnet.nearblocks.io/txns/CU4UED48FSL6W25JCfwPMn3eLxbHWL8BvUuz9qHWEYGZ) |
| 7 | Record the honest fix (judge verdict: accept, Lean proof checked) | recorded | [`FwDVK8Dg…`](https://testnet.nearblocks.io/txns/FwDVK8DgFEN7HB8xPTG1THfYp8VABv9s6pZPB57Yqx6v) |
| 8 | Install the honest fix | success | [`EjW7Qs26…`](https://testnet.nearblocks.io/txns/EjW7Qs26QVQuNiEvfEBLjzkQrxVLUtUMrmcCDNEw8Ekm) |
| 9 | Replay the same overflow attack | rejected (`Guard spending denied`); recipient balance unchanged | [`mTeyRshJ…`](https://testnet.nearblocks.io/txns/mTeyRshJKfpdykMWfYUwkpGgYN6JEc9h2rmAneKR8CY) |
| 10 | Legitimate spend 100 | success | [`GJPsKCjN…`](https://testnet.nearblocks.io/txns/GJPsKCjNjYuWwWvFX6HYhyLmh9cTioVkeGsAoExgxZeN) |

Final `guard_state`: `spent` 200 (wrapped by the step-3 overflow), `total_paid` 18446744073709551816 yoctoNEAR, `pending_paid` 0, one entry in `guard_history`. The script also compared the recipient's on-chain balance with these payments.

## Check it yourself (read-only)

```sh
scripts/judge-check.sh --stage a --allow-testnet-read
```

A3 reads `guard_state` and `get_fix_proof` from `fixseal-guard.testnet` over public RPC and checks that the installed policy hash and accept record match the bundled evidence. No keys or signing. Expected on 2026-11-10: `PASS A3`.

## Limits

The contract trusts the record submitter (the registered agent) and the owner; it does not run Lean. Attestation is mock. The Lean-to-contract correspondence is checked on 1,028 fixed inputs, not proved. Transaction links point to a third-party explorer; the hashes can also be queried on any testnet RPC.
