# AGENTS.md — Corliss contributor guide

This file is the single canonical instruction file for AI coding agents
working in this repository.

## Project overview

Corliss is a library that provides interoperability (read and write) with Inventor Files (.ipt, .iam, .idw)


## Code style guidelines

- Functions: 4-20 lines. Split if longer.
- Files: under 500 lines. Split by responsibility.
- One thing per function, one responsibility per module (SRP).
- Names: specific and unique. Avoid `data`, `handler`, `Manager`.
  Prefer names that return <5 grep hits in the codebase.
- Types: explicit. No `any`, no untyped functions.
- No code duplication. Extract shared logic into a function/module.
- Early returns over nested ifs. Max 2 levels of indentation.
- Exception messages must include the offending value and expected shape.
- Match the surrounding file's conventions: naming, comment density,
  module structure. Comments explain *why*, never restate *what*.
- Small, scoped, behavior-preserving changes. No adjacent feature work,
  no opportunistic refactors, no speculative abstractions or new public
  surface without a shipped caller, persisted data, or an explicit
  requirement.
- New features get a dedicated directory at `docs/features/` for all their supporting documents.
- Remove throwaway experiment implementations and their branches/worktrees once the
  investigation concludes; retain findings and measurements in `docs/features/`,
  not dormant experiment code. Preserve unrelated unfinished feature work.
- No dead code or half-built public surface. Future work is documented in
  `docs/features/` design notes, not shipped as unreachable stubs.
- Prefer explicit fallbacks over `unwrap`, `expect`, or `unreachable!` in
  runtime paths; panics are acceptable in tests only.
- `unsafe` is forbidden by workspace lint; do not add it. The only exception
  is described in "Unsafe code exception" below.
- Typed boundaries are load-bearing: IDs and dialects are parsed/normalized once and reused.
- Keep CLI commands thin: parse args, resolve config once, call typed
  library functions, render output.
  - Keep your own comments. Don't strip them on refactor — they carry
  intent and provenance.
- Write WHY, not WHAT. Skip `// increment counter` above `i++`.
- Docstrings on public functions: intent + one usage example.
- Reference issue numbers / commit SHAs when a line exists because
  of a specific bug or upstream constraint.

## Testing instructions

- Tests run with a single command: `cargo test`. `cargo xtask pr` runs the
  PR gate (lint of changed files, then clippy, build and tests of the affected
  packages) exactly as CI runs it; run it before every push.
- Set up or update the toolchain and native dependencies with the installer
  for your OS (`install-linux.sh`, `install-macos.sh`, `install-windows.ps1`).
- Every new function gets a test. Bug fixes get a regression test.
- Mock external I/O (API, DB, filesystem) with named fake classes,
  not inline stubs.
- Add focused regression tests for every bug fix and behavior change.
  Parsers, ID derivation, and math especially.
- Filesystem tests use temp dirs or injected roots (`tempfile`); never
  depend on the real user home directory being writable.
- Tests that build throwaway git repos must not inherit the contributor's
  machine config: pass `--no-gpg-sign` on every fixture commit (a global
  `commit.gpgsign = true` otherwise signs as the fixture's fake identity and
  fails), and build `git2` signatures with a fixed `Signature::now(...)`,
  never `repo.signature()`. CI cannot catch either — its runners have no
  global gitconfig, so the breakage only ever shows up on a developer's box.
- Tests must be F.I.R.S.T: fast, independent, repeatable,
  self-validating, timely.
- Integration tests live under `tests/`; the `xtask` crate keeps its tests in
  its own modules
- Integration / E2E tests are placed in tests

## Formatting

- Use the language default formatter (`cargo fmt`). Don't discuss style beyond that.

## Logging

- Structured JSON when logging for debugging / observability.
- Plain text only for user-facing CLI output.

## Repository
- Keep good branch and worktree hygene, when work is merged delete those.
- Millestones and Issues are tracked on this repo with gh.
- All feature work should have a matching feature typed issue on gh.
- Bugs found must be catalogued as a Bug typed gh issue and.
- Bugs fixing work must come from a Bug typed gh issue.

### Change and replacement

- Follow semantic versioning. A breaking change needs a version bump and changelog migration notes.
- Separate structural refactors from numerical or behavioral changes.
- Pin current behavior with owned fixtures before a refactor; a refactor passes them unchanged.
- A behavior change needs independent expected results, a kernel decision, and a new behavior version.
- Keep earlier behavior selectable for replay until a recorded migration.
- Reuse existing adaptors, builders, history modules, and mesh factories.
- Give each replacement a deletion task, corpus gate, and one-milestone migration limit.
- Prove one complete operation before extending a replacement engine.
- Do not add Go-kernel code or production fallbacks; check Go-derived work against its known defects.

### Operation result contracts

- Return completion, diagnostics, validation coverage, history, and achieved error together.
- Return typed errors with operation, entities, failure class, and cause; never only a flag or a string.
- Distinguish completed empty results from failed, cancelled, or incomplete computations.
- Validate topology, geometry consistency, self-intersection, and tolerance consistency in order.
- Select validation invariants for the declared body kind.
- Make the check mode explicit; a sampled pass is not an exact pass.
- Offer local checks of changed entities; keep the full ordered check for publication.
- Treat missing or inconclusive required checks as incomplete, never as valid.
- Keep invalid and partial geometry out of successful operation results.
- Express history as deleted, merged, created, split, or derived relations with all parents.
- Compose operation, copy, and healing history; leave document naming policy to the application.
- Perform healing explicitly on a copy and return its identity remap.

### Ownership and thread safety

- Published results are immutable, `Send`, and `Sync`; reading them takes no lock.
- Mutate only in operation-local builders, before publication.
- Make operations non-destructive by default; test input isolation after success, failure,
  cancellation, and source mutation.
- Define ownership for all geometry, topology, caches, and meshes reachable from a result.
- Do not treat `Clone`, `Arc`, locks, or `&Shape` as proof of isolation.
- Keep no thread-local or global mutable algorithm state; pass state in an operation context.
- A result must not depend on the thread, earlier work on it, or concurrent operations.
- Make operation objects `Send`; avoid `Rc` and `RefCell` in operation state.
- Give each `unsafe impl Send/Sync` a safety comment and a Miri or loom test.
- Assert `Send`/`Sync` at compile time for each public type, or document why it is thread-confined.
- Key reusable results by input revisions, options, and tolerance policy.
- Merge parallel results in a total order; no output may depend on hash iteration order.
- Measure copy, allocation, and lock costs before redesigning storage.

### Verification and completion

- Use independent oracles: closed forms, error-bounded integration, Monte Carlo membership,
  invariants, and reviewed baselines.
- Change a baseline only by a reviewed update that states the reason and behavior version.
- Require invariants, independent expected results, and generated degenerate cases for changed algorithms.
- Check per-face geometry, trims, tolerances, and history; volume alone is insufficient.
- Compare two processes byte for byte; compare multi-thread with single-thread results.
- Run concurrency stress tests and ThreadSanitizer for code that claims thread safety.
- Record revisions, options, platforms, oracle kinds, executed tests, and exclusions.
- Keep kernel acceptance and application acceptance separate.
- Benchmark identical accepted workloads against the previous release; report time and peak memory.
- Remove superseded production paths when their replacement gate passes.
- Do not weaken tests, widen tolerances, or hide failures to complete a migration.

## Integration workflow

- Use `main` as the integration branch. Create each integration pull request with
  `gh` and merge it with `gh` after all required checks are green.
- The coordinating agent serializes merges, resolves conflicts, and runs the
  relevant checks. A subagent's work is complete only after its committed changes
  are integrated into `main`.
- After integration and checks, delete the completed worktree and its merged
  branch. Verify that all commits are on `main`, local changes are accounted for,
  and no active process or pending task needs the checkout or its build artifacts.
- Start new worktree branches from current `main`. Preserve unfinished work and
  report its branch, worktree, and remaining failures during handoff.


## Security considerations

- **Never commit secrets.** Keep real tokens out of docs, fixtures, and
  tests. No secret scanner runs in CI yet.
- **Sanitization is the trust boundary:** all untrusted hook payload text
  passes through the `ai-memory-hooks` sanitizer before storage; do not
  create paths that bypass it (or hook backpressure, or the single-writer
  actor).
- **Capture exclusions** (`[capture] ignore_paths` in the nearest
  `.ai-memory.toml` marker) drop recognized file-tool events before they
  reach spool, transport, logs, or storage — preserve this behavior in
  native hook commands and generated integrations.
- **Dependency policy:** run `cargo deny --all-features check` and
  `cargo audit` (both installed by the installers) when you add or update a
  dependency; they are not CI gates yet. Do not add dependencies without
  checking the project doesn't already have the capability, and match
  existing versions/idioms.
- **Destructive operations** (`purge-project`, `reset`, `restore`) must
  keep their confirmation flags and live-process checks.

## Project maintenance rules

- **CHANGELOG is a merge gate.** Any change affecting user-visible
  behavior, installation, supported platforms/agents/providers,
  deployment, env/config, or public tool/admin surfaces must add a
  `CHANGELOG.md` entry under `## [Unreleased]` (correct Keep-a-Changelog
  heading — `Added`/`Changed`/`Deprecated`/`Removed`/`Fixed`/`Security` —
  past-tense, trailing `(#NNN)` reference) and update the relevant
  README/docs references in the same commit. Internal refactors and
  test-only churn are exempt.
  `cargo xtask release` reads the release's `## [X.Y.Z]` section and refuses
  a version bump smaller than its headings require, so file each entry under
  the heading that matches its semver impact. Released sections are frozen.
- **Run the humanizer on new or heavily edited prose.** New documentation,
  and any large rewrite of existing text (a new doc section, a rewritten guide,
  a long CHANGELOG entry, an RFC or design note), goes through the `humanizer`
  skill (a maintainer-side skill; contributors without it leave the pass to
  review) before it is committed: it strips AI-writing patterns while keeping
  every fact. Prose only — code blocks, commands, paths, link targets and
  headings (anchors) stay unchanged, and a rewrite that adds or drops a fact is
  a defect. Released CHANGELOG sections are frozen and are not rewritten.
- **CI pacing: full matrix on every pull request and before release.** CI is one
  workflow, `.github/workflows/corliss.yml`, and every check in it is a
  `cargo xtask` command that runs the same way locally. A pull request runs
  PR validation (`cargo xtask pr`) on Linux x86_64 and the full matrix (Linux
  x86_64, macOS x86_64 and arm64, Windows x64; `cargo xtask full`); `main` is
  protected and merges only when all five checks pass. The full matrix also
  runs on a `v*` tag push and on a manual dispatch in `release` mode. The
  workflow does not run on branch pushes: after pushing a branch directly,
  dispatch it on that SHA. Running the full matrix is **mandatory right
  before a release**: dispatch the workflow with `mode=release` and the
  version on the exact release-candidate SHA, and wait for green before
  tagging. Never tag a release whose SHA lacks a green full
  matrix.
- **Reclaim build storage after every release — run `cargo clean`.** 
  Once a release is tagged and its artifacts are published, run
  `cargo clean` (and `cargo clean` in each release worktree / extra target dir
  you created, then remove finished `git worktree`s). This is a mandatory
  post-release cleanup step, not optional housekeeping. It also removes the
  rehearsal packages in `target/dist/`. During normal development, a weekly
  `cargo clean` is the lighter-touch equivalent.
- **No version bumps or release tags without explicit user approval.**
  Do not bump crate/package versions automatically.
- **Non-negotiable quality bar.** No malicious or obfuscated code, no
  regressions (add tests when unsure), no stale docs, no security or quality
  degradation. Any one blocks a merge regardless of approval; when in doubt,
  double-check and live-test with the automated checks.
- **PR evaluation:** audit each PR (pros, cons, recommended fix) and carry
  approved tickets through yourself: resolve soft blockers (rebase or base
  retarget, missing tests/docs/CHANGELOG, small fixes and easy decisions) as
  maintainer commits; ask the author only about a hard blocker you cannot
  resolve. Once the maintainer approves a batch, push each green task to its
  branch as it finishes. Version bumps and tags always need explicit
  approval.
- **Semantic versioning:** patch = fixes; minor = additive (new CLI
  subcommands, MCP tools, config keys, a new agent harness or LLM
  provider); major = breaking (on-disk format without migration, removed
  subcommands, breaking MCP schema changes, big rewrites).
- **Release cadence and ordering.** Batch tickets by semver impact and
  ship fixes as a patch release promptly — never let a bug fix wait on
  unreleased feature work. The `[Unreleased]` section signals the bump:
  only `### Fixed` → patch; any `### Added` → minor; anything breaking →
  major. Bucket incoming work at triage with the `breaking-change` label
  and version milestones.
- **Branch model.** `main` is the patch line: it takes fixes and docs only
  (`[Unreleased]` holds only `### Fixed`), and patch tags are cut from it.
  Additive work targets the standing release candidate branch. Check the
  latest published tag and remote release branches before choosing a PR base;
  do not infer the current release from a branch name. A candidate branch
  does not authorize a version bump, tag or publication.
  Keep `main` an ancestor of it by forward-merging
  `main` into it after every fix lands, never the reverse; when a patch
  release ships, rebuild the branch's `[Unreleased]` as only its own entries
  so released sections stay identical to `main`'s. A fix lands on `main`
  first. To ship the minor, merge the candidate branch into `main`, run the release
  gates on that SHA and tag from `main`; that candidate branch then goes
  dormant. A PR's base branch follows its semver bucket: fixes → `main`,
  features → the standing candidate branch.