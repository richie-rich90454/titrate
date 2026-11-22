# Gallery assets

The SVGs in `docs/assets/` are terminal-style captures of real program output. They are generated, not drawn by hand.

Regenerate from the repository root:

```powershell
.\scripts\capture_demos.ps1
```

This requires `target/debug/trc.exe` (build with `.\scripts\build.ps1`). The script runs each demo through the bytecode VM, captures its stdout, renders a dark terminal card as SVG, and exits. It opens no windows, starts no servers, and uses no browser automation. Output is deterministic: CRLF is normalized to LF, no timestamps or environment-dependent paths are embedded, and files are written as UTF-8 without a BOM.

Captured programs: `examples/hello.tr`, `examples/fibonacci.tr`, `demos/collatz.tr`, `demos/caesar_cipher.tr`, and `demos/csv_to_json.tr` (run against `archive/old_test_files/data.csv`). Outputs longer than 40 lines end with an explicit truncation marker. All text inside each card is verbatim VM stdout.

`docs/assets/site-stats.svg` is different: its five counts are computed from the source tree when the script runs (library modules, stdlib tests, demos, examples, registered natives in `lookup.rs`). The three verified facts are fixed results from prior runs. Because the counts are a snapshot, `--check` will correctly report staleness if counted files change.

CI can assert that the committed images are current:

```powershell
.\scripts\capture_demos.ps1 --check
```

The check regenerates everything in memory, compares against the committed files, and exits non-zero if any are missing or stale. `-Check` is equivalent.
