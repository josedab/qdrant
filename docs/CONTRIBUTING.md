# Contributing to Qdrant
We love your input! We want to make contributing to this project as easy and transparent as possible, whether it's:

- Reporting a bug
- Discussing the current state of the code
- Submitting a fix
- Proposing new features

## We Develop with GitHub
We use github to host code, to track issues and feature requests, as well as accept pull requests.

## We Use [GitHub Flow](https://guides.github.com/introduction/flow/index.html), So All Code Changes Happen Through Pull Requests
Pull requests are the best way to propose changes to the codebase (we use [GitHub Flow](https://docs.github.com/en/get-started/quickstart/github-flow)). We actively welcome your pull requests:

1. Fork the repo and create your branch from `dev`.
2. If you've added code that should be tested, add tests.
3. If you've changed APIs, update the documentation and API Schema definitions (see [development docs](https://github.com/qdrant/qdrant/blob/master/docs/DEVELOPMENT.md#api-changes))
4. Ensure the test suite passes.
5. Make sure your code lints (with cargo).
6. Issue that pull request!

## Any contributions you make will be under the Apache License 2.0
In short, when you submit code changes, your submissions are understood to be under the same [Apache License 2.0](https://choosealicense.com/licenses/apache-2.0/) that covers the project. Feel free to contact the maintainers if that's a concern.

## Report bugs using GitHub's [issues](https://github.com/qdrant/qdrant/issues)
We use GitHub issues to track public bugs. Report a bug by [opening a new issue](https://github.com/qdrant/qdrant/issues/new/choose); it's that easy!

## Write bug reports with detail, background, and sample code

**Great Bug Reports** tend to have:

- A quick summary and/or background
- Steps to reproduce
  - Be specific!
  - Give sample code if you can.
- What you expected would happen
- What actually happens
- Notes (possibly including why you think this might be happening, or stuff you tried that didn't work)

## Code Coverage

Qdrant uses automated code coverage metrics to ensure test quality and prevent regressions.

### Coverage Requirements

- **Minimum threshold:** 70% overall coverage
- **PR impact:** Coverage should not decrease significantly (>5%) for changed files
- **Coverage runs:** Automatically on all PRs and pushes to master

### Running Coverage Locally

To generate and view coverage reports locally:

```bash
# Run unit tests with coverage
RUN_PER_PACKAGE=true tools/unit-test-coverage.sh

# Run integration tests with coverage (requires Python/Poetry setup)
tools/integration-test-coverage.sh

# View HTML report (after running coverage)
open target/llvm-cov/html/index.html
```

### Coverage Tools

The project uses **cargo-llvm-cov** for coverage generation:
- Separates unit and integration test coverage
- Generates lcov format for Codecov integration
- Supports per-package runs for low-memory environments

### CI Coverage Checks

Coverage is automatically checked on every PR:
1. **Unit test coverage** - Tests individual modules and functions
2. **Integration test coverage** - Tests full system behavior
3. **Merged coverage report** - Combined view uploaded to Codecov
4. **Threshold check** - Fails if coverage < 70%

### Viewing Coverage Reports

- **Codecov dashboard:** View detailed coverage at https://codecov.io/gh/qdrant/qdrant
- **PR comments:** Codecov bot automatically comments on PRs with coverage changes
- **Local reports:** Generated HTML reports show line-by-line coverage

### Best Practices

1. **Add tests for new code:** Aim for high coverage of new features
2. **Test critical paths:** Prioritize coverage for core functionality
3. **Review coverage reports:** Check which lines are untested before submitting PR
4. **Don't game the metrics:** Focus on meaningful tests, not just coverage numbers

### Excluded Files

The following files are automatically excluded from coverage:
- Generated code (protobuf definitions)
- Debug utilities (`src/wal_inspector.rs`, `src/schema_generator.rs`, etc.)
- Build scripts and tooling

## Use a Consistent Coding Style

If you are modifying Rust code, make sure it has no warnings from Cargo and follow [Rust code style](https://doc.rust-lang.org/1.0.0/style/).
The project uses [rustfmt](https://github.com/rust-lang/rustfmt) formatter. Please ensure to run it using the
```cargo +nightly fmt --all``` command. The project also use [clippy](https://github.com/rust-lang/rust-clippy) lint collection,
so please ensure running ``cargo clippy --workspace --all-features`` before submitting the PR.

## License
By contributing, you agree that your contributions will be licensed under its Apache License 2.0.

