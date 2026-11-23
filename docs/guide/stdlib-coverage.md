# Standard library coverage report

`docs/stdlib-coverage.json` is a generated inventory of public declarations in
`lib/tt/**/*.tr`. It is not a hand-maintained checklist.

## What the report is, and what it is not

The report answers one narrow question: which public `fn`, `class`, `interface`,
and `enum` declarations exist in the standard library source today? Every count
is computed from the files found under `lib/tt` at generation time. No counts
are hardcoded in the generator.

The report does **not** answer whether those declarations are correct, tested,
exposed on all three backends, or equivalent to any external API. It is an
inventory, not proof of parity with the reference lists named in
`ROADMAP.md` section 21. In particular, it makes no claim of Java, Python, or
NumPy parity, and a high API count is not a 100% coverage result. Closing the
gap between this inventory and a reference list remains the work described in
`ROADMAP.md` phases 9–13.

## Regenerating

Run from the workspace root with PowerShell 7 (`pwsh`):

```
pwsh scripts/gen-stdlib-coverage.ps1
```

The script writes `docs/stdlib-coverage.json`. It reads only `lib/tt/**/*.tr`;
it does not run the compiler or any tests. Output is deterministic: given the
same source tree, repeated runs produce byte-identical JSON (files are sorted
with an ordinal comparison, and no timestamp is recorded).

If a new top-level directory is added under `lib/tt`, the script stops with an
error naming that directory. Add it to the domain mapping in the generator
before regenerating; an unmapped directory is never silently dropped.

## Statuses

| Status | Meaning in this report |
|---|---|
| `implemented` | A public `fn`, `class`, `interface`, or `enum` declaration with that name exists in the listed `.tr` file at the listed line. |

That is the only status emitted today. It is a source-presence marker, not a
quality gate. It does not imply:

- a `.tr` test exists for the API;
- a generated doc page exists for it;
- a cross-backend parity test passes;
- a capability tag or benchmark has been added;
- behavior matches a CPython, Java SE, NumPy, or any other reference API.

`partial`, `missing`, and `explicitly-excluded` appear in `ROADMAP.md` section
21 as the target vocabulary for a future reference-list audit. They are not
produced by this generator, because comparing against a reference list requires
a reviewed mapping that does not exist yet. Adding those statuses means adding
a real comparison, not relabeling the present inventory.

## Fields

Each API entry carries `name`, `kind`, `module`, `source`, `line`, `status`, and
`referenceList`. The `referenceList` value is the label for the API's domain
from the `ROADMAP.md` section 21 table; it identifies which reference set the
domain is ultimately meant to be compared against. Its presence does not mean
that comparison has been done.

Each domain carries `moduleCount` (number of `.tr` files mapped to it) and
`apiCount` (number of public declarations found in them). Top-level `totals`
repeats the sums.

## Current inventory

Snapshot from the first generated report (571 `.tr` files, 10,957 public
declarations). Consult the JSON for current figures:

| Domain | Modules | APIs |
|---|---:|---:|
| Core/util | 278 | 5,306 |
| Collections | 46 | 918 |
| Data/science | 92 | 1,227 |
| Network/async | 63 | 1,431 |
| System/OS | 66 | 1,460 |
| Graphics | 11 | 362 |
| Audio | 9 | 157 |
| ML | 6 | 96 |
| **Total** | **571** | **10,957** |

These numbers describe source size only. They will change whenever `.tr` files
are added, removed, or edited; regenerate the JSON rather than editing it or
this snapshot by hand.
