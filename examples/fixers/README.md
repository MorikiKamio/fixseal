# Real AI fixers, blind run

These are post-experiment records of **simulated environment, synthetic data**. Devin (swe-2-high) and Codex (gpt-6.1-sol) worked in answer-withheld copies, using UInt64/Bool-only Target arithmetic without network access. See the original [Devin record](../../docs/fixer/FX3-devin.md) and [Codex record](../../docs/fixer/FX3-codex.md).

| Agent | Attempt 1 | Final | Final Proof.lean lines |
| --- | --- | --- | --- |
| Devin (swe-2-high) | PROOF_REJECTED: invalid rewrite argument | accept | 10 |
| Codex (gpt-6.1-sol) | PROOF_REJECTED: unfinished equivalence | accept | 19 |
| DeepSeek V4.1 Flash via NEAR AI Cloud¹ | accept (after local Lean revisions) | operator rejudge: accept | 27 |

Both attempts pass attacks 0/12 and regressions 16/16. Final line counts include the import and blank lines. The kernel checked the accepted proofs against the fixed specification; the judge reran fixed tests. No chain result is claimed.

Each attempt directory holds only Target.lean and Proof.lean. Attempt 2 is transcribed from the recorded final source. Complete original attempt-1 proofs were not retained in the supplied records: Devin's is reconstructed by reversing the documented rewrite edit, and Codex's by restoring the final `omega` in the failing branch. These reproduce the recorded errors, but their byte-for-byte historical identity is unconfirmed.

From the repository root, with the installed tools and cached dependencies described in [JUDGES.md](../../JUDGES.md):

```sh
cargo test --manifest-path judge/Cargo.toml --locked --offline --test fixers
```

The tests load each archived pair, combine it with the fixed spend reference inputs and byte-identical baseline regressions, judge both attempts, and verify their bundles. They assert the exact reasons, verdicts, behavioral counts and proof status. This is a local replay of recorded work, not another blind experiment.

When preparing an independent fixer copy, exclude `examples/fixers/`, `docs/fixer/` and every `after_honest` reference directory. The leak scanner excludes these post-experiment archives; this exclusion does not make a public export suitable for a blind run.

¹ [NX1 provenance](../../docs/fixer/NX1-nearai.md): 120 turns, no `done`, wrapper attempts 0; two candidate shell judgements accepted under spend-3. The operator rejudged the same copy. `spend/nearai-deepseek-v4.1-flash/final/` contains byte-identical final sources from the supplied blind copy, not a reconstruction. The third test replays this final source under current spend-4 and verifies its bundle (0/12 attacks, 16/16 regressions, proof=true, 1,028 export checks). The caps and completion handling differ from FX3; these are not model rankings. All figures describe a **simulated environment, synthetic data**.
