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

## Reproduce

Run from the repository root with the installed toolchains:

```sh
export PATH=/root/.cargo/bin:/root/.elan/bin:$PATH
cargo build --release
cargo test
cargo run --release --bin gen0-bench
sh bench/prepare-search.sh
cargo run --release --manifest-path bench/search/Cargo.toml
```

Both commands accept `--corpus`, `--results`, `--lean`, `--timeout-ms`, and
`--minif2f-decision` after Cargo's `--`. Run `sh bench/verify.sh` for all
formatting, build, test, and Clippy checks. No command installs anything.
Generated root `target/` and `Cargo.lock` are deliberately untracked in this
lane; retain the core lane's manifest/package name/lockfile during integration.

`bench/prepare-search.sh` fetches the exact GitHub core commit recorded in
`bench/core-revision.txt` into an ignored detached worktree. The separate
`bench/search/Cargo.toml` compiles that engine's unchanged `search.rs`,
`budget.rs`, and `tactics.rs` with this lane's checker and reporting module.
It has no dependencies and does not add core-owned files to this branch.
The driver checks that the core worktree remains clean and pinned before each
run. Rebuild after changing either source checkout.

The search driver uses the core's ordered `DEFAULT_TACTICS`, not corpus hints.
It records the exact tactics and limits in `execution_path`: 32 maximum nodes,
5,000 ms per search, beam width 8. Each Lean subprocess has its own 10,000 ms
default timeout. The synchronous search deadline is checked between calls;
one in-flight check can finish after the search deadline. Actual elapsed times
and actual checker-call node counts are preserved.

## Coordinator integration

- Keep the exact shared `CheckResult`/`ProofChecker` definitions once.
- Add `mod lean; mod bench;` wherever the core binary/library exposes modules.
- Keep the standalone `gen0-bench` Cargo target pointing at `bench/main.rs`.
- `bench/search_adapter.rs::run_search(checker, corpus, tactics, cfg)` calls the
  actual shared `search()` API. Its theorem argument is the declaration without
  `:=`; its successful proof is the complete returned Lean source.
- After merging the core sources, replace the three `core-checkout` module paths
  in `bench/search/main.rs` with the integrated source paths and update the
  revision check/provenance accordingly. The pinned driver remains independently
  reproducible until then. Do not silently label a different engine revision
  as the recorded one.

## Result interpretation

Every JSON row represents an executed theorem attempt. Direct checks use null
node counts. Search rows preserve every engine failure transcript (including
the exact candidate source and raw Lean stdout/stderr), even if a later
candidate succeeds. Successful rows retain the full checked proof source.

On a failed theorem the harness also runs `trace_state` on the original
declaration and stores it as `initial_goal`, with a separate duration. This is
explicitly the initial goal, not an invented intermediate search state. These
diagnostic checks do not add to search node counts or proof totals. Timeouts or
startup errors can prevent Lean from emitting a goal; their raw errors remain
available. JSON preserves diagnostic text; Markdown provides a readable table
and failed-theorem transcripts.

Lean is invoked with `-DwarningAsError=true`, so ordinary `sorry`/`admit` proofs
fail by exit status. This checker accepts trusted benchmark source; it does not
sandbox arbitrary Lean elaborator code, ban axioms, or prevent source from
changing Lean options.
