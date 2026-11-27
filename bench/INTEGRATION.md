# Generation 0 benchmark lane

This lane owns `src/lean.rs`, `src/bench.rs`, `bench/`, benchmark results, and
their tests. `src/checker.rs` repeats the exact shared checker contract, as
explicitly requested in the lane brief; deduplicate it during integration.

The checkout began without the core search engine or a Cargo manifest. The
standalone benchmark target lives under `bench/` to avoid taking ownership of
`src/main.rs`. Preserve that target when integrating the core lane's manifest.
There are no external Cargo dependencies.

The direct benchmark checks each corpus theorem's supplied tactic hint. It
measures Lean verification of known proofs, not autonomous proof discovery.
Direct results must have null node counts and identify their execution path.
Search measurements require the core lane's actual `search()` implementation;
no substitute engine or fabricated search results belong in this lane.

miniF2F provenance, fetch evidence, and the run decision live under
`bench/minif2f/`. A skipped dataset has no score or pass rate.
