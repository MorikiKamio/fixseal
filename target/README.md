# Synthetic path target: English summary

This is the historical VM subject, a **simulated environment, synthetic data**, with no actual file I/O. Each `path.fn` is interpreted by the [judge](../judge/README.md): `before` decodes/joins/normalizes and can escape the root; `after_honest` checks containment; `after_hardcode` blocks only the public attack; `after_weaken_test` rejects everything and removes normal regressions. The judge always reruns baseline regressions and rejects test modifications. Use `cargo run --manifest-path judge/Cargo.toml --locked --offline -- target after_honest`. The primary kernel-checked subjects live under `lean`.

## Detailed reference

# Synthetic path join target

For a **simulated environment, synthetic data**. Each `path.fn` implements path joining in the [judge instruction language](../judge/README.md); no real files are opened.

| Version | Behavior |
| --- | --- |
| `before` | URL decode, join and normalize; can return paths outside the allowed root. |
| `after_honest` | Checks root containment after normalization. |
| `after_hardcode` | Rejects only the public attack string. |
| `after_weaken_test` | Rejects every input and removes normal regressions. |

`before/tests/regression.json` contains the original regressions used by the judge. Other versions retain that file except `after_weaken_test`, which modifies it. Shared `corpus.json` and `policy.json` are trusted inputs. Attack paths are fictional and contain no real data.
