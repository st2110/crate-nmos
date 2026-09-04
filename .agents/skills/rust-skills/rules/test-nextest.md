# test-nextest

> Use `cargo-nextest` as a faster, more reliable test runner

## Why It Matters

`cargo-nextest` runs each Rust test as a separate process instead of sharing a single process per test binary. This gives better isolation, clearer failure output, faster parallel execution, and first-class CI features such as retries, flaky-test detection, JUnit reports, and per-test overrides. It is a drop-in replacement for `cargo test` in most cases.

## Installation

```bash
cargo install cargo-nextest --locked
```

## Basic Usage

```bash
# Run all tests (replaces cargo test)
cargo nextest run

# Run tests matching a pattern
cargo nextest run parse

# Run a specific test
cargo nextest run -- parse::handles_empty_input

# List tests without running
cargo nextest list
```

## Project Configuration

Add repository-wide settings in `.config/nextest.toml`:

```toml
[profile.default]
# Stop on first failure locally
fail-fast = true

# Treat tests running longer than 60s as slow
slow-timeout = "60s"

[profile.ci]
# Run all tests regardless of failures in CI
fail-fast = false
# Retry flaky tests up to 2 times
retries = 2
```

Use a profile:

```bash
cargo nextest run --profile ci
```

## Filtering Tests

Nextest has its own expression language for selecting tests:

```bash
# By test name
cargo nextest run -E 'test(auth)'

# By binary
cargo nextest run -E 'binary(my_app)'

# By package
cargo nextest run -E 'package(my_crate)'

# Compound expression
cargo nextest run -E 'test(auth) and not test(slow)'

# Unit tests only (inside lib)
cargo nextest run -E 'kind(lib)'

# Integration tests only
cargo nextest run -E 'kind(test)'
```

## Retries and Flaky Tests

Configure retries to surface intermittent failures without masking them:

```toml
[profile.default]
retries = 2
```

A test that fails and then passes on retry is reported as **flaky**:

```
FLAKY 2/3 [   0.002s] my_crate::auth test_expired_token
```

To fail the entire run on flaky tests (recommended in CI once flakiness is unacceptable):

```toml
[profile.ci]
retries = 2
flaky-result = "fail"
```

Use per-test overrides only when a known subset of tests is unstable:

```toml
[[profile.default.overrides]]
filter = 'test(known_flaky)'
retries = 4
```

## Common Mistakes

1. **Relying on retries instead of fixing flakiness.** Retries help detect flakiness; they do not fix the underlying race, shared port, timeout, or global state bug.
2. **Running benchmarks as tests.** `cargo nextest run` does not run `[[bench]]` targets. Use `cargo bench` for benchmarks.
3. **Ignoring slow-timeout warnings.** A test marked slow is often a test that will eventually flake or deadlock. Investigate rather than raising the timeout blindly.
4. **Global state leaks.** Nextest isolates tests at the process level, but tests that write to shared files, environment variables, or network ports can still interfere. Keep tests hermetic.
5. **Assuming doctests run by default.** Nextest does not run doctests. Run them separately with `cargo test --doc` if your project relies on them.

## When Not to Use Nextest

- You rely heavily on doctests as the primary test mechanism.
- Your tests depend on the exact `cargo test` output format.
- You are running tests under a custom test harness that nextest does not support.

## CI Integration

Nextest produces machine-readable reports:

```bash
# JUnit XML for CI dashboards
cargo nextest run --profile ci --junit-xml results.xml
```

This makes flaky tests, slow tests, and failure patterns visible in CI without parsing human-readable output.

## See Also

- [test-criterion-bench](test-criterion-bench.md) - benchmarking with Criterion
- [perf-profile-first](perf-profile-first.md) - profile before optimizing
- [test-descriptive-names](test-descriptive-names.md) - write clear test names
- [test-arrange-act-assert](test-arrange-act-assert.md) - structure tests with AAA
