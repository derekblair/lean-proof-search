# Generation 0 measured reproduction summary

Two independent direct runs proved 25/25 supplied hints. Two independent actual core search runs proved the same 20/25 theorems and returned identical successful proof sources. Search node counts vary with the wall-clock deadline; they are preserved per run, not normalized.

Harness source: `625d6bfbf29d01c72c0372c5444b398a329d08f4`. Core source: `29576aa8718e657a9918f12aed02d4cce14e5f46`. Both checkouts were clean for measurement.

| Mode / run | Proved | Total check/search ms | Wall ms including goal probes | Nodes | JSON | Summary |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Direct 1 | 25/25 | 4627 | 4641 | not applicable | [gen0-1795753148063-36911.json](gen0-1795753148063-36911.json) | [Markdown](gen0-1795753148063-36911.md) |
| Direct 2 | 25/25 | 4736 | 4751 | not applicable | [gen0-1795753152859-37933.json](gen0-1795753152859-37933.json) | [Markdown](gen0-1795753152859-37933.md) |
| Search 3 | 20/25 | 33344 | 34258 | 179 | [gen0-1795753157796-38582.json](gen0-1795753157796-38582.json) | [Markdown](gen0-1795753157796-38582.md) |
| Search 4 | 20/25 | 33389 | 34332 | 178 | [gen0-1795753208099-46235.json](gen0-1795753208099-46235.json) | [Markdown](gen0-1795753208099-46235.md) |

## Full corpus: repeat-run measurements

All direct checks passed. Search failures below exhausted the configured search time budget, not the Lean subprocess timeout. Direct rows have no search node counts.

| Theorem | Direct ms | Search proved | Search ms | Search nodes | Winning search tactic |
| --- | ---: | --- | ---: | ---: | --- |
| bool_not_not | 188 | True | 374 | 2 | simp |
| eq_symmetry | 198 | True | 588 | 3 | omega |
| eq_transitivity | 183 | True | 577 | 3 | omega |
| exists_witness | 189 | True | 393 | 2 | simp |
| list_append_nil | 194 | True | 357 | 2 | simp |
| list_length_append | 204 | True | 362 | 2 | simp |
| list_nil_append | 183 | True | 183 | 1 | rfl |
| nat_add_assoc | 184 | True | 587 | 3 | omega |
| nat_add_cancel | 229 | True | 588 | 3 | omega |
| nat_add_comm | 189 | True | 571 | 3 | omega |
| nat_add_zero | 178 | True | 183 | 1 | rfl |
| nat_le_succ | 224 | True | 373 | 2 | simp |
| nat_lt_succ | 184 | True | 362 | 2 | simp |
| nat_mul_one | 194 | True | 382 | 2 | simp |
| nat_numeral_sum | 191 | True | 189 | 1 | rfl |
| nat_zero_add | 179 | True | 384 | 2 | simp |
| option_map_id | 194 | True | 362 | 2 | simp |
| prop_and_comm | 178 | False | 5042 | 27 | none |
| prop_and_left | 188 | False | 5022 | 27 | none |
| prop_and_right | 183 | True | 383 | 2 | simp |
| prop_false_elim | 178 | True | 388 | 2 | simp |
| prop_identity | 184 | True | 388 | 2 | simp |
| prop_imp_trans | 179 | False | 5139 | 27 | none |
| prop_or_comm | 183 | False | 5062 | 27 | none |
| prop_or_left | 178 | False | 5150 | 28 | none |

## Failure evidence

Five searches remained unproved: prop_and_comm, prop_and_left, prop_imp_trans, prop_or_comm, prop_or_left. Recorded candidate diagnostics include unsolved goals, inapplicable rfl/constructor/apply/rewrite tactics, simp making no progress, omega finding no usable constraints, decide rejecting free variables, type mismatches, and unknown h/n identifiers. All raw candidate sources, stdout, and stderr are retained in the search JSON files. Initial-goal probes are separately recorded and timed. No checker timeout occurred in these benchmark runs.

Checker tests also exercise missing executables, timeout cancellation, process-group cleanup, large stdout/stderr, concurrency, and rejection of ordinary sorry/admit via Lean warnings-as-errors.

## miniF2F

NOT RUN. The OpenAI repository was fetched at 4e433ff5cadff23f9911a2bb5bbab2d351ce5554, and five original validation excerpts were retained. It requires Lean 3.42.1 and legacy Mathlib. The community clone returned Repository not found, the installed Lean 4.34.1 could not import Mathlib, and a bounded Lake update failed because the fetched upstream has no Lake configuration. No Mathlib or toolchain was installed. No miniF2F score or pass rate is claimed. [Evidence](../bench/minif2f/decision.txt).

## Validation and integration

Both release builds passed. Root cargo test: 17 passed (8 harness + 9 checker); search adapter cargo test: 2 passed. Formatting checks and Clippy with -D warnings passed for both targets. Cargo metadata confirms no external dependencies in either manifest.

[Raw build/test/lint output](validation-20261127T041906Z.log). [Reproduction commands and coordinator integration](../bench/INTEGRATION.md).

The coordinator still needs to integrate the core manifest and shared checker definition, and redirect the search driver to integrated core modules. This branch already measured the actual pinned search() implementation through an ignored detached Git worktree; it does not contain copied or reimplemented engine source. No PR was opened.

Measured JSON files are unchanged. Markdown-only blank-line whitespace was normalized after measurement; the renderer received the equivalent formatting fix. Final validation after that fix is in [this log](validation-final-20261127T042116Z.log).
