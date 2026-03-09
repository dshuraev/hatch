# Contributing

## Scope

This project follows the working agreement in `AGENTS.md`:

* keep changes small and demonstrable
* prefer the minimum code needed to satisfy the story
* use Conventional Commits
* keep trunk green

## Tooling

Required local tools are pinned in `mise.toml`. Install them with:

```bash
mise install
```

That installs:

* Rust `1.89.0`
* Rust `nightly-2025-10-31` for `miri` and fuzzing
* `task` `3.48.0`
* `cargo-deny`
* `cargo-auditable` for release builds

Optional tools for heavyweight verification:

* `cargo-audit`
* `cargo-bloat`
* `cargo-geiger`
* `cargo-llvm-cov`
* `cargo-fuzz`
* `cargo-coupling`

## Development Workflow

1. Make the smallest testable change.
2. Add or update tests under `test/unit/` or `test/integration/` as appropriate.
3. Install or update the pinned toolchain:

   ```bash
   mise install
   ```

4. Run the fast verification gate:

   ```bash
   task ci
   ```

5. Before opening a release-oriented or riskier change, run the heavyweight suite:

   ```bash
   task check
   ```

## Task Reference

Common commands:

```bash
task format
task clippy
task test
task ci
task release
task check
```

Informational reports:

```bash
task geiger
task bloat
task coverage
task coupling
```

## CI Policy

GitHub Actions defines two repository workflows:

* `CI`: runs on pull requests and pushes to `master`
* `Release`: runs when a `v*` tag is pushed

The blocking CI gate is intentionally narrow:

* `cargo fmt --all --check`
* `cargo clippy --all-targets --all-features -- -D warnings`
* `cargo test --all-targets --locked`
* `cargo deny check`
* `cargo build --release --locked`

Heavyweight analysis stays outside the default PR gate because it is slower and less deterministic.

## Dependency And Supply Chain Checks

`cargo-deny` is the primary blocking dependency policy tool. It currently enforces:

* RustSec advisories and yanked crates
* wildcard dependency bans
* unknown registry and git source rejection
* approved license policy

`cargo-audit` remains part of the release verification path for an additional advisory check.

`cargo-auditable` is required for release artifacts so distributed binaries retain dependency metadata for downstream scanning.

## Release Process

1. Ensure the version and documentation are ready to ship.
2. Run:

   ```bash
   task release
   ```

3. Create and push a tag in the form `vX.Y.Z`.
4. The release workflow will build:
   * `dist/hatch`
   * `dist/hatch-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
   * `dist/sha256sums.txt`

`cargo-bloat` is not a blocking release step, but it should be reviewed before shipping if binary size is important for the target environment.

## Commit Format

Use Conventional Commits:

```text
feat(scope): add digest-first handshake
fix(store-sqlite): handle WAL rotation
docs(guide): expand replay walkthrough
```

Add a `BREAKING CHANGE:` footer when necessary.
