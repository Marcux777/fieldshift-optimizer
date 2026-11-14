<!-- AI-assisted (OpenBot agent) -->
# Technical Migration Report: Python/Streamlit -> Rust + SvelteKit 5

- **Baseline Commit (`main`):** `16cc391318b57fbf44a31c75c994fbb0683b3115`
- **Target Branch:** `feat/rust-svelte-migration`
- **Protocol:** `docs/EVALUATION_PROTOCOL.md` v1.1 (`ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9`)

## 1. Scientific Parity Summary

| Artifact / Metric | Python Oracle (`16cc391`) | Rust Implementation (`feat/rust-svelte-migration`) | Parity Classification |
|---|---|---|---|
| Feasible plans (`AD1..AD6` clayey) | `169, 289, 400, 576, 729, 961` | `169, 289, 400, 576, 729, 961` | Exact (`IDENTICAL`) |
| Default catalog hash (`AD4 + argilosa`, `576` plans) | `52cb1c7d8970745d8dbdd56e57f10d3db1b3dc5e33d8ab2b202531750809096d` | `52cb1c7d8970745d8dbdd56e57f10d3db1b3dc5e33d8ab2b202531750809096d` | Exact (`IDENTICAL`) |
| Indicator tensor (`576 × 43 × 3 = 74,304` `f64` values) | `fixtures/reference/components_default.json` | `0` bit mismatches (`max |Δ| = 0.0`) | Bit-for-bit (`IDENTICAL`) |
| Policy selections (`P0, P1, P1b, P2` × 15 weights) | `fixtures/reference/selections_15w.json` | Identical plan IDs and tie-break keys in all 60 combinations | Bit-for-bit (`IDENTICAL`) |
| Pareto frontier & alternatives | `36` vectors, `50` plans (`14` equivalents), `6` distinct alternatives | `36` vectors, `50` plans (`14` equivalents), `6` distinct alternatives | Bit-for-bit (`IDENTICAL`) |
| Canonical `RunResult` SHA-256 | `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66` | `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66` | Bit-for-bit (`IDENTICAL`) |
| Bootstrap indices (`seed=20261114`, `10,000 × 7`) | `70,000` indices (`PCG64` + `SeedSequence`) | `70,000 / 70,000` exact matches | Bit-for-bit (`IDENTICAL`) |
| Primary test outcome (`P2` vs `P1b`) | `NEGATIVO` (`0.1744` vs `0.1466`, `D = +0.0064`, CI `[-0.0609, +0.0375]`, `p = 0.6875`) | `NEGATIVO` (`0.1744` vs `0.1466`, `D = +0.0064`, CI `[-0.0609, +0.0375]`, `p = 0.6875`) | Bit-for-bit (`IDENTICAL`) |
| Canonical `EvalReport` SHA-256 | `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18` | `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18` | Bit-for-bit (`IDENTICAL`) |

## 2. PR #1 Audit Fixes Applied

1. **Removed Frozen Report Replay:** Replaced PR #1's static file reader in `backend/src/main.rs` with live computation across `fieldshift-io`, `fieldshift-model`, and `fieldshift-eval`.
2. **Exact Shewchuk Summation (`fsum`):** Replaced Neumaier summation in `selection.rs` with CPython's exact Shewchuk `math.fsum` algorithm (`fieldshift_numeric::num::fsum`).
3. **Call-Site Rounding Parity:** Distinguished `np.round(v, 9)` (`rint(v * 10^9) / 10^9`) in `q()` and `objective_vectors` from Python's exact decimal `round(v, 9)` in `select_alternatives` (`optimization.py:384, 418, 440`).
4. **Pareto Vector Grouping:** Grouped non-dominated plans after rounding objective vectors to 9 decimals (`36` vectors, `50` plans with `14` equivalents), matching `optimization.py`.
5. **Pairwise NumPy Reductions (`np_sum`):** Implemented NumPy's 8-accumulator pairwise summation (`PW_BLOCKSIZE = 128`) so threshold comparisons (`sum(P) < 20.0`) match to the last ULP.
6. **Exact NumPy PCG64 & SeedSequence:** Implemented 4-word `SeedSequence` mixing, `PCG64` XSL-RR 128/64 with 32-bit buffered uint32 generation, and Lemire bounded sampling.

## 3. Mutation Controls (M-1 and M-2)

- **M-1 (Stale Cache on Same Label, Changed Climate):** Verified in `fieldshift-eval/tests/differential_eval.rs` (`m1_same_label_different_climate_invalidates_cache`); isolated mutation disabling content hash check fails as required (`exit 101`).
- **M-2 (Ignoring Missing `T2M` in Completeness Check):** Verified in `fieldshift-eval/tests/differential_eval.rs` (`m2_missing_t2m_excludes_both_overlapping_pairs`); isolated mutation omitting `t2m` check yields 28 instead of 26 pairs and fails (`exit 101`).
