# Egglog Debug Dumps

This project has an opt-in debug mode to inspect the egglog program emitted during the
LLHD unit roundtrip (LLHD Module -> Egglog Program -> LLHD Module). The dump is
zero-cost at runtime when the feature is disabled.

## What it emits

When enabled, the test writes a human-readable dump per unit that includes:
- Unit metadata (name, kind, signature)
- Serialization schema notes (Unit, ArgValue, Inst, etc.)
- CFG skeleton schema
- The exact egglog facts emitted by `unit_to_egglog_program`

## How to run

Enable the cargo feature and set the env var to turn on dumping.

### cargo test

```bash
LLHD_EGGLOG_DEBUG=1 cargo test -F egglog-debug --test roundtrip_unit_egglog
```

### cargo nextest

```bash
LLHD_EGGLOG_DEBUG=1 cargo nextest run -F egglog-debug --test roundtrip_unit_egglog
```

## Output location

Dumps are written to:

```
target/egglog/
```

Each unit gets its own file with a stable name derived from the test file, unit name,
and unit index, for example:

```
tests__my_unit__0.egglog.txt
```

The test prints a single line like this once per run:

```
egglog debug dumps: target/egglog
```

## Notes

- The dump hook runs only when the feature is enabled and `LLHD_EGGLOG_DEBUG=1` is set.
- If the test is filtered out (no tests run), no dumps are produced.
