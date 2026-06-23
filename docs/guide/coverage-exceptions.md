# Coverage Exceptions

This page tracks source files whose line coverage is **below 100%** and the
reason each gap is accepted (for now) or scheduled to be closed.

The table is regenerated whenever `pipette coverage` (or
`scripts/coverage-baseline.ps1`) is run. When you close a gap, move the row
to the "Closed gaps" section with the PR that fixed it.

## Active exceptions

| File | Covered | Total | % | Reason | Tracking |
|------|--------:|------:|--:|--------|----------|
| _(none yet — run `pipette coverage` to populate)_ | — | — | — | — | — |

> **Format:** one row per file. `Reason` is a short phrase such as
> `error-path only`, `unreachable defensive code`, `platform-gated`, or
> `todo: add tests`. `Tracking` is an issue number or PR link.

## Closed gaps

| File | PR | Notes |
|------|----|-------|
| _(none yet)_ | — | — |

## Acceptance policy

- **100% is the target.** Every line of compiler, VM, and native-backend
  Rust code should be exercised by a test.
- **Defensive `unreachable!` / panic paths** may be listed here with the
  reason `unreachable defensive code` rather than tested, but they must be
  genuinely unreachable — a `debug_assert!` or comment must justify it.
- **Platform-gated code** (e.g. Windows-only syscall shims) is listed with
  reason `platform-gated` and covered on the relevant platform.
- **Anything else** is a `todo: add tests` and should have an issue.

## Regenerating this table

```bash
pipette coverage
# then transcribe the below-100% rows from coverage-summary.txt
```

See [Code Coverage](./coverage) for the full workflow.
