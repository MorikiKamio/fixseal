# Recording the live split-screen take

**Simulated environment, synthetic data**: disposable testnet accounts, mock attestation, no real funds. One `guard_init` per contract, so each account gives exactly one take — record in one pass.

## Prepare (operator, off camera)

1. Create three disposable contract accounts into the legacy keychain:
   `create-live-accounts.sh` (faucet; prints nothing key-like).
2. For each `fixseal-live-N.testnet`, deploy + register + add the restricted key:
   `FIXSEAL_LIVE_CONTRACT=fixseal-live-N.testnet setup.sh`
   The add-key step grants `guard_spend` on that contract only. The same owner, agent and recipient accounts as the recorded run are reused.
3. Optional live-fixer take first: `scripts/live-fix.sh` runs the repair agent
   (`FIXSEAL_FIXER_CMD`, default `codex exec -s workspace-write` with the FX3
   instruction) in a blind copy, judges each attempt, and on accept writes
   `demo-artifacts/live-fix/accepted.json`.

## The take

Left half of the screen — terminal:

```sh
FIXSEAL_LIVE_CONTRACT=fixseal-live-1.testnet FIXSEAL_TESTNET_STEP_PAUSE=4 ./run.sh
```

`run.sh` resolves the spend key whose FunctionCall receiver is that contract, then `scripts/testnet-guard.sh --execute` prints one headline per transaction, e.g.:

```text
[3/10] Overflow attack: spend 18446744073709551116 … PAID (the bug); tx=…
[9/10] Replay the same overflow attack … refused: Guard spending denied; tx=…
```

To install the freshly accepted fixer bundle instead of the fixed `after_honest`, the operator may instead run:

```sh
scripts/testnet-guard.sh --execute --accepted-bundle demo-artifacts/live-fix/accepted.json
```

Right half of the screen — the viewer, live:

```text
https://<pages-url>/index.html?contract=fixseal-live-1.testnet&live=1
```

(or the local file). `?contract=` accepts any valid testnet account id; `?agent=` selects the agent account. Polling is every 3 s and pauses while the tab is hidden. The Activity feed at the top is derived only from state diffs: payments, a wrapped spent counter, new judge records, and the rule replacement. Refused transactions never appear — they change no state; they show in the terminal and the explorer. Non-default contracts keep the code-hash and permission checks and mark repository-evidence checks as n/a.

Suggested beats: init + 600 + the overflow attack (feed shows the over-cap payment), record/install rejections (terminal only), record accept + install (feed: record added, rule replaced), replay attack refused (terminal only), legitimate 100 (feed: spent counter).

## After

`demo-artifacts/testnet-guard-<contract>/` holds the evidence table, bundles and final `guard-state.json`. Use a fresh `fixseal-live-N` for each retake; `guard_init` runs once per contract.
