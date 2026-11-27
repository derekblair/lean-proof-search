# lean-proof-search

A zero-dependency Rust proof-search engine that learns to prove Lean theorems by self-play, with every proof machine-checked by Lean and every generation scored on public benchmarks.

## How it works

1. **Propose.** Gemini Neon (XHigh) agents propose tactic sequences and proof sketches for a problem already stated in Lean, from public benchmarks (miniF2F, PutnamBench) and open-problem lists.
2. **Search.** A zero-dependency Rust engine runs the search: it keeps the proof state, tries branches in parallel, prunes dead ones, and tracks time and node budget.
3. **Verify.** Lean checks every candidate. Only a Lean-checked proof counts. Every failure is saved with the exact goal state, so the failure mode can be inspected instead of guessed.
4. **Learn.** Successful proof traces become training data for ranking the next tactics, and the next generation reruns the same benchmark. Strength climbs generation by generation.
5. **Record.** A `paper.pdf` records each generation, the experiments run, and the results, so it stays the ground truth instead of context slop.

The human's job in the loop is scheduler: data generation, search, and core optimization run in parallel on different machines, coordinated rather than done one at a time.

## Why it's needed

The big automated provers are model-heavy, mostly closed, and glued together in Python. There is no clean, open, reproducible search engine where Lean is the verifier and the whole thing gets stronger by self-play.

Derek's math work today is also one theorem at a time, hand-orchestrated. A proof does not make the next proof easier except through a lessons file. This engine is meant to make it compound, and to make the score legible: a public benchmark score, a Lean-checked proof, and a paper, instead of a PR count.

## Design principles

1. Lean is the arbiter, not the model. A result counts as total or partial Lean verification, and the status is published with the claim. Overnight output goes into a verification queue before it counts.
2. The harness is the product, not the prompt. The durable edge is the harness, tooling, retries, search, and checking, built by trial and error, not a single clever prompt.
3. Generate overnight, verify in the morning. Generation is cheap and parallel; verification is the bottleneck, so design around it.
4. Open source the loop so it compounds. A public, checkable artifact draws contributors and independent extensions; a private demo does not.
5. Optimize a measurable score. Every generation is scored on the same public benchmarks, by pass rate, time, and cost, so progress is a number, not a feeling.
6. Prefer a minimal checkable artifact. A smaller, single-file, Lean-checked proof is easier to audit, reproduce, and build on than a large one.

## Status

Generation 0. Sprint plan in progress. No engine code yet.
