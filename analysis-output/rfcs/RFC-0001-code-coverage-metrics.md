# RFC-0001: Code Coverage Metrics in CI

**Status:** Draft
**Author:** Analysis Team
**Created:** 2025-11-16
**Category:** Quick Win
**Estimated Effort:** 3-5 dev-days

---

## Summary

Integrate automated code coverage metrics into the CI/CD pipeline to improve test quality visibility, prevent regressions, and guide testing efforts.

---

## Motivation

### Current State
- Qdrant has comprehensive test suites (~500+ tests)
- No automated coverage metrics
- Coverage blind spots unknown
- Difficult to assess PR test quality

### Problems Solved
1. **Unknown coverage gaps**: Can't identify untested code paths
2. **Regression risk**: No automatic check for coverage decreases
3. **Testing guidance**: Hard to prioritize test writing
4. **Quality metrics**: No quantitative measure of test completeness

### User Impact
- **Developers**: Confidence in changes, clear testing targets
- **Maintainers**: Data-driven test strategy
- **Contributors**: Understand where tests are needed

---

## Detailed Design

### 1. Tool Selection

**Recommended:** `cargo-tarpaulin`

**Why not alternatives?**
- `cargo-llvm-cov`: Less mature, complex setup
- `kcov`: Requires DWARF debug info, slower
- **tarpaulin**: Rust-native, good CI integration, actively maintained

**Installation:**
```toml
# .github/workflows/coverage.yml
- name: Install tarpaulin
  run: cargo install cargo-tarpaulin
```

---

### 2. CI Integration

**New workflow:** `.github/workflows/coverage.yml`

```yaml
name: Code Coverage

on:
  pull_request:
    branches: [ master, dev ]
  push:
    branches: [ master ]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo
            target/
          key: ${{ runner.os }}-coverage-${{ hashFiles('**/Cargo.lock') }}

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin --locked

      - name: Generate coverage
        run: |
          cargo tarpaulin \
            --workspace \
            --exclude-files 'src/schema_generator.rs' 'src/wal_inspector.rs' \
            --timeout 600 \
            --out Xml \
            --output-dir coverage

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/cobertura.xml
          fail_ci_if_error: false

      - name: Check coverage threshold
        run: |
          COVERAGE=$(grep -oP 'line-rate="\K[0-9.]+' coverage/cobertura.xml | head -1)
          THRESHOLD=0.70  # 70% minimum
          if (( $(echo "$COVERAGE < $THRESHOLD" | bc -l) )); then
            echo "Coverage $COVERAGE is below threshold $THRESHOLD"
            exit 1
          fi
```

---

### 3. Coverage Thresholds

**Initial thresholds** (conservative):
```yaml
coverage:
  global:
    target: 70%    # Overall codebase
    threshold: 5%  # Allow 5% drop

  per_crate:
    segment: 75%   # Core functionality
    collection: 70%
    storage: 70%
    api: 80%       # API should be well-tested
```

**Enforcement:**
- **PRs**: Fail if coverage drops > 5% for changed files
- **Master**: Warning if global coverage drops
- **Trend**: Track coverage over time

---

### 4. Reporting

**Codecov dashboard:**
- Line coverage by file/module
- Branch coverage
- Coverage diff on PRs
- Sunburst visualization

**PR comments:**
```markdown
## Code Coverage Report

**Overall coverage:** 72.4% (+1.2%)

### Changes
- ✅ `lib/segment/src/index/hnsw_index/hnsw.rs`: 78% → 82%
- ⚠️ `lib/collection/src/shards/mod.rs`: 65% → 62% (-3%)

**Files with < 60% coverage:**
- `lib/collection/src/shards/transfer/mod.rs`: 45%

[View full report](https://codecov.io/...)
```

---

### 5. Exclusions

**Automatically exclude:**
- Generated code (`*.rs` from protobufs)
- Debug utilities (`src/segment_inspector.rs`, etc.)
- Main entry point (mostly setup)
- Unsafe FFI bindings (test separately)

**Configuration:**
```toml
# .tarpaulin.toml
[config]
exclude-files = [
    "src/schema_generator.rs",
    "src/wal_inspector.rs",
    "src/wal_pop.rs",
    "lib/api/src/grpc/qdrant.rs",  # Generated
]

exclude-unsafe = false  # We want to see unsafe coverage
```

---

## Example Usage

### Developer Workflow

1. **Before PR:**
   ```bash
   cargo tarpaulin --workspace --out Html
   open target/tarpaulin-report.html
   # Identify gaps, add tests
   ```

2. **During PR review:**
   - Codecov bot comments on PR
   - Reviewer sees coverage impact
   - Discuss if drop is acceptable

3. **Continuous improvement:**
   - Monthly review of uncovered code
   - Prioritize critical paths

---

## Implementation Plan

### Phase 1: Setup (Week 1, Days 1-2)
- [ ] Install and test tarpaulin locally
- [ ] Create `.github/workflows/coverage.yml`
- [ ] Set up Codecov account
- [ ] Configure exclusions

### Phase 2: Integration (Week 1, Days 3-4)
- [ ] Run baseline coverage report
- [ ] Set initial thresholds
- [ ] Enable PR checks
- [ ] Document in CONTRIBUTING.md

### Phase 3: Iteration (Week 2, Day 5)
- [ ] Tune thresholds based on data
- [ ] Fix CI performance issues
- [ ] Create coverage badges for README

---

## Backwards Compatibility

✅ **No breaking changes**
- Purely additive CI feature
- Doesn't affect runtime code
- Optional for local development

---

## Alternatives Considered

### Alternative 1: Manual Coverage Reports

**Pros:**
- No CI overhead

**Cons:**
- Requires manual effort
- Inconsistent
- No PR integration

**Verdict:** ❌ Rejected

---

### Alternative 2: Only Track Critical Modules

**Pros:**
- Faster CI
- Focused effort

**Cons:**
- Blind spots in other code
- Partial picture

**Verdict:** ❌ Rejected (start comprehensive, tune later)

---

### Alternative 3: Separate Coverage Job (On-Demand)

**Pros:**
- Doesn't slow down every PR

**Cons:**
- Easy to forget
- No automatic checks

**Verdict:** ⚠️ Possible future optimization

---

## Open Questions

1. **Performance impact on CI?**
   - **Answer:** Run on separate job, parallel to tests
   - **Mitigation:** Cache tarpaulin binary, use `--release` mode

2. **What threshold is realistic?**
   - **Answer:** Start at 70%, adjust based on baseline
   - **Data needed:** Run baseline report first

3. **How to handle flaky tests?**
   - **Answer:** Mark flaky tests, exclude temporarily
   - **Process:** Track in GitHub issues

---

## Success Metrics

### Quantitative
- **Baseline coverage:** X% (measure in Phase 1)
- **Target (6 months):** +10% from baseline
- **PR coverage drops:** < 5% of PRs cause >5% drop
- **CI time increase:** < 10% overhead

### Qualitative
- Developer feedback: "Coverage reports helped me find gaps"
- Maintainer feedback: "Easier to assess PR quality"

---

## Rollback Strategy

If coverage CI causes issues:
1. Disable PR failure (make informational only)
2. Reduce to weekly runs instead of per-PR
3. Remove entirely (revert workflow)

**Risk:** Low (non-invasive change)

---

## Follow-up Work

After initial implementation:
1. **RFC-0001-A**: Branch coverage (not just line coverage)
2. **RFC-0001-B**: Integration test coverage separation
3. **RFC-0001-C**: Coverage visualization in docs

---

## References

- **cargo-tarpaulin**: https://github.com/xd009642/tarpaulin
- **Codecov**: https://codecov.io/
- **Rust coverage guide**: https://doc.rust-lang.org/rustc/instrument-coverage.html

---

**Related RFCs:**
- RFC-0009: Integration Test Framework

**Approvals Required:**
- [x] Engineering Lead
- [ ] CI/CD Owner
