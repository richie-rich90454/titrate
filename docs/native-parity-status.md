# Native stdlib parity status

This is the machine-readable tracker for the remaining native stdlib parity
gaps. `scripts/classify-native-failures.ps1` counts failures in a
`.local/*.log` harness log and emits a Markdown table with counts and
percentages; this document holds the qualitative root-cause checklist. Counts
are deliberately not hardcoded here: regenerate them from a fresh log.

Native stdlib parity is not a gating test. `stdlib_runtest_native` is
`#[ignore]`d and runs report-only in CI; see `HANDOFF.md` §6 and `ROADMAP.md`
§6 and §8. Do not claim parity from this document alone.

## Refresh the counts

```powershell
pwsh scripts/classify-native-failures.ps1 .local/native_dump_run.log
```

With no argument the script picks the newest `native*.log` under `.local/`.
Redirect stdout to a `.md` file to archive a snapshot. The harness writes a
full dump when `STDLIB_NATIVE_DUMP` is set; see `trc/tests/stdlib_runtest.rs`.

## Root-cause checklist

Status key: **Fixed** = the root cause was removed by the audit fix; **Open**
= still a known gap. Fixed items stay in the tracker so a regression is
visible when the same signature reappears in a fresh log. Statuses record the
root-cause audit at the time of writing and must be re-verified by a fresh run.

- [x] **Iterator_default LNK1120 (linker)** — **Status: Fixed.** A generic
  default method body emitted no symbol for implementers of the iterator
  interface, producing an unresolved external at link time. The headless
  emission fix removed it. Watch for `LNK1120`/`LNK2019` in fresh logs.
- [ ] **`.clone()` / LNK** — **Status: Open.** Cloning a generic container can
  still leave an unresolved native symbol at link time. Tracked separately
  from Iterator_default because the call site and fix are different.
- [ ] **Cast failures** — **Status: Open.** `return type cast failed` and
  `cannot convert value of type 'X' to 'ptr'` at native call sites remain.
- [x] **Variant private fields** — **Status: Fixed.** Private field access
  through a `Variant` no longer produces a codegen failure.
- [x] **StructExt** — **Status: Fixed.** The struct-extension packing and
  unpacking path no longer produces the recorded native failure.
- [ ] **fn-field (function-valued fields)** — **Status: Open.** Calls through
  a function-valued field still fail to extract or bitcast the function
  pointer on some receivers.
- [ ] **Crashes / nonzero exit codes** — **Status: Open.** Native binaries
  still exit with fault codes (for example access violations) on a subset of
  programs. Safety takes priority over wrong-output gaps.
- [x] **Audio ABI** — **Status: Fixed.** The audio native ABI mismatch behind
  the recorded failures was corrected.
- [ ] **Stack overflows** — **Status: Open.** Recursion and mangling
  collisions can still overflow the native stack.

## Mapping to the classifier

| Script category | Audit item(s) |
|---|---|
| LNK1120 linker | Iterator_default LNK1120 (fixed), `.clone()` / LNK (open) |
| unsupported call target | fn-field (open); related unresolved call sites |
| unsupported expression | `codegen: unsupported expression` signatures |
| unsupported assignment target | `codegen: unsupported assignment target` signatures |
| cast failures | Cast failures (open) |
| crashes (exit codes) | Crashes (open), stack overflows (open) |
| FAIL lines | Audio ABI (fixed) and other wrong-output cases |
| other | Vtable and synthetic-main failures not covered above |

A category dropping to zero confirms the signature is gone from that log; it
does not prove a fix unless a regression test accompanies it. When an item is
fixed, add a named regression test and flip its checkbox here in the same
change.
