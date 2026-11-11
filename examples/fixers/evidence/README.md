# Packaged evidence

These are **simulated environment, synthetic data**.
The two honest bundles are the offline T010 outputs from 2026-11-10, using
path-7 and spend-4. They contain source as byte arrays, the full report and its
SHA-256. Neither contains a chain readback. They are curated copies of generated
artifacts; the original run files and historical host logs are preserved.

`scripts/judge-check.sh --stage a` checks this `SHA256SUMS` manifest, which covers
the two bundles, both specifications, FIX5/T008 historical host logs and the
recorded AI attempts/provenance. A2 independently recomputes input, report and
expression hashes in Python. These checks establish local integrity only; a
manifest shipped alongside its files does not authenticate their origin.

`scripts/judge-check.sh --stage c` runs the current judge, kernel checks and fixed
tests and replays both bundles (C4). The accepted reports must reproduce as
`VERIFIED`; inspect the verdict as well. The fixed specifications still require
operator content/hash review. FIX5 predates spend-4 and the guard; it does not
establish current guard payments or wasm/source correspondence.

The archive's first proofs are reconstructions from recorded errors and edits.
This replay is not a new blind experiment. These public bundles include answers;
exclude the enclosing `examples/fixers/` archive with every `after_honest` directory and the post-experiment records
when preparing an independent fixer copy. See [archive provenance](../README.md).

T013 appends three integrity entries for the curated NX1 record and exact final
Target/Proof sources; the previous sixteen entries and bundle bytes are
unchanged. NX1 historical judgement used spend-3, with no done call and an
operator rejudge; current C5 replays its final source with spend-4. Raw API logs
are excluded from this evidence set.
