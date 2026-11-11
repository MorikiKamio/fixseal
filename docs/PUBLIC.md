# Public export procedure

**Distribute only a fresh, checked output of `export-public.py`. Never publish the development tree, a bulk git snapshot or an old export.** The development tree retains internal tasks, QA reviews, operations logs and local settings. Ignore rules are a secondary safeguard and do not remove indexed files or history. The public allowlist is authoritative; new candidates require explicit inclusion.

In the development copy, choose a new local directory for each submission:

```sh
python3 scripts/export-public.py demo-artifacts/submission-current
python3 scripts/export-public.py --check demo-artifacts/submission-current
```

Existing output is never overwritten. Run `--check` immediately before distribution; after source changes, create a new export. It compares the exact allowlist, all current bytes after wasm sanitization, and rejects extra/missing files, symlinks and stale content with exit 2. Export checks relative paths, symlinks, private home paths and internal identifiers before creating output. Neither command commits, pushes or sends data.

Included files are implementation, tests, pinned Lean inputs, synthetic targets, lockfiles, notices and reproduction documents. Internal task/QA/design/progress/HANDOFF files, raw operations logs, local agent settings, secrets and build caches are excluded. Explicitly allowlisted host evidence includes [FIX5](demo-output-host-fix5.txt): operator run on 2026-11-10, path-7/spend-3, both Lean builds and ten matched readbacks/replays, demo exit 0. The operator reports 66 judge tests and 72 contract tests passing separately from this demo-only log. The earlier [T008 host record](demo-output-host-t008.txt) remains available as historical evidence. Reference evidence is preserved unchanged.

Two upstream test wasm files are sanitized only on export: 72 home-path occurrences in data/custom sections are replaced at the same length. Code, offsets and original files are preserved; execution of sanitized wasm remains unconfirmed. The existing SBOM also has a sanitized upstream path. This procedure does not erase information from the development copy, index or history.

Post-experiment `examples/fixers` and `docs/fixer` records are public, including clearly marked reconstructed first attempts. They contain answers. Independent fixer copies must exclude both directories and every reference `after_honest` directory. Scanner exclusions do not authorize shipping answers in a blind copy.

License inventory remains incomplete (R5-C05): CC-BY-4.0 full text/attribution for the distributed version, LLVM-exception dependency/NOTICE/count reconciliation, current judge/transitive dependencies, Lean/toolchain distribution scope and wasm/source provenance need review. The LLVM exception text already exists; do not describe it as missing. Existing LICENSE/NOTICE/third-party notices are retained. No external license retrieval was performed. The detailed historical inventory follows.


T011 adds the one-command reviewer script/helper/tests, the separate pinned
review Dockerfile/entrypoint, its ignore file, the review guide, two curated
path-7/spend-4 offline bundles and their evidence manifest. All are explicitly
allowlisted. Submission drafts/payload, task results and progress stay internal.
The bundles contain answers and live under the already excluded
`examples/fixers/evidence/` post-experiment archive. Keep the existing
post-experiment/reference exclusions for an independent fixer copy. A1/A2
check saved integrity only; C4 reruns the proof and fixed tests. CI and Docker
remain unexecuted, with a first-run placeholder in the READMEs.

T013 adds the standard-library `fixers/nearai/` tool and offline tests, the
CLI summary test, and the curated NX1 public record/final source archive (eight
explicit paths). Raw API transcripts, judgement logs, local paths, task results
and submission drafts stay internal. NX1 ran on spend-3 and did not call done;
current tests replay the exact final sources on spend-4. The fixer must run only
against an answer-withheld copy; a public export includes answers. Unit tests
use temporary synthetic copies, without sockets or API calls. Historical pricing
was not retained, so no NX1 USD amount is exported.

## Detailed reference and license inventory

# Public export scope

The development copy retains internal tasks, QA originals and operator records. Export only files listed in development `scripts/public-files.txt`, rather than submitting the entire workspace.

```sh
# Run in the development copy; no external transmission
python3 scripts/export-public.py demo-artifacts/submission-current
python3 scripts/export-public.py --check demo-artifacts/submission-current
```

Use a new destination for each submission and run `--check` immediately before submitting. If the example destination exists, choose another. This read-only check compares the allowlist, every file byte (after wasm anonymization), extra files and symlinks; mismatches exit 2. Regenerate after source changes. Existing `submission/` directories may be stale; do not submit them unchanged.

Default output is ignored `submission/`. Existing directories are never overwritten; choose a new destination. Before export, the tool checks file existence, relative paths, symlinks, personal home paths and internal project identifiers.

Included: implementation, tests, fixed Lean inputs, dummy targets, lockfiles, licenses and reproduction guides. Excluded: internal tasks / qa / progress / HANDOFF / design, operator shell scripts, raw logs, local agent settings, secret `.env` and build caches. Operator success logs replace only personal paths with `<repo>` and state versions/provenance in README. Export tooling, allowlist and their tests are development administration files.

The 2 test wasm files embed upstream home paths in data/custom sections. Export alone replaces them with equal-length build paths (72 occurrences), preserving originals, code/other sections, length and offsets. Chain execution of anonymized resources was unconfirmed because of bind restrictions.

1 upstream absolute path in the SBOM is also anonymized. Existing SBOM / third-party licenses cover the upstream template, not the full current judge / Lean project. Incomplete third-party license warnings remain. LICENSE / NOTICE and attribution are retained.

Export creates a local artifact; it does not change the git index, commit, push or transmit anything. It also does not erase internal information from the development copy or git history.

T008 exports include 2 `docs/fixer/` records, `examples/fixers/` attempts/guides and replay tests. These post-experiment records anonymize personal paths and state first-Proof reconstruction and provenance limits. Always exclude `examples/fixers/`, `docs/fixer/` and every reference `after_honest/` from blind fixer copies. Public copies include answers and cannot be given directly to fixers. Scanner archive exclusions do not authorize inclusion in blind copies.

Development `.gitignore` also excludes internal paths, but does not remove indexed files. Regardless of index state, do not commit/push the development copy; move only a checked export into a separate public workspace. Ignore rules do not erase index/history entries.

## T005: Outstanding third-party license and attribution work

Only existing inventories, SBOM and local caches were inspected, with no downloads/installations. The following distribution checks remain; do not invent license text or claim the inventory is complete.

| Item | Observed state | Missing / next check |
| --- | --- | --- |
| `caniuse-lite` 1.0.30001727 / CC-BY-4.0 | 1 SBOM entry. `LICENSE-THIRD-PARTY.txt` section 12 has only a summary/original URL. Neither root nor frontend node_modules has the LICENSE. | Obtain complete official text and version-specific attribution/notices; compare the matching distribution in an environment permitting downloads. |
| Apache-2.0 WITH LLVM-exception | Section 13 already includes Apache text and 2 LLVM exception paragraphs. Cached `wasmtime` 36.0.9 / `cranelift-codegen` 0.123.9 / `target-lexicon` 0.13.5 LICENSE files also contain the exception. The leading "used by 0 / missing" conflicts with this. | Text is present. Still map SBOM dependencies/versions to LICENSE / NOTICE / copyright, actual distributed resources and inventory counts. Do not merely delete the warning. |
| LLVM exception scope | Existing SBOM declares it for Cranelift / Wasmtime, pulley, regalloc2, target-lexicon, wasm-encoder, wasmparser, wasmprinter, rustix, etc.; some offer MIT / Apache alternatives. | Do not exclude these as "0 entries". Reconcile lockfiles and distributed resources in a complete attribution list; old SBOM counts alone do not establish current scope. |
| Judge Rust direct/transitive dependencies | `judge/Cargo.lock` exists; existing SBOM predates judge additions. | Inventory current locked dependencies, collect version-specific originals/attribution/NOTICE and map them to the third-party list. Some cached texts do not establish completeness. |
| Lean 4.34.1 / standard library / native toolchain | Versions are pinned and must be provisioned. | Decide source-only versus bundled binaries, then inventory texts/attribution/resources for that scope. Existing SBOM does not cover them. |
| Test wasm / built contract | Upstream test notices retained; path anonymization preserves code/offsets. | Map binaries to sources/build versions/attribution and test anonymized execution. Supplied contract wasm/current-source correspondence was also unconfirmed. |

JUDGES.md / AGENTS.md / `.github/workflows/ci.yml` and the path-5 host reference log were added to the allowlist. CI includes dependency provisioning for future runners; this task did not access the network or run CI.
