# LLHD CFG Skeleton + Egglog E-Graph Integration Summary

This document summarizes the recent additions to LLHD for integrating a CFG skeleton and an egglog e-graph alongside each unit.

## Overview

- Each unit now stores transient CFG skeleton and e-graph data in parallel with existing IR.
- The e-graph captures non-stateful (pure) DFG operations using the egglog schema from `Wirelog/resources/egglog/llhd_dfg_sort.egg`.
- The CFG skeleton anchors control flow, side effects, and phi semantics as block arguments referencing e-classes.
- Data is transient and not serialized into LLHD assembly or binary formats.

## New Data Structures

- `CfgSkeleton` and related types in `llhd/src/ir/cfg_skeleton.rs`.
- `UnitEGraph` and helpers in `llhd/src/ir/egraph.rs`.
- `UnitData` now holds:
  - `cfg_skeleton: CfgSkeleton`
  - `egraph: UnitEGraph`
  Both are marked with `#[serde(skip, default)]` to keep them transient.

## Behavior

- Pure opcodes are embedded as `LLHDDFG` nodes in the e-graph.
- Side-effecting and control-flow ops are emitted in the CFG skeleton, referencing e-classes for operands.
- Phi nodes are lowered to block arguments, with per-edge argument lists on CFG skeleton terminators.
- Non-pure results are represented in the e-graph as `ValueRef` leaves to preserve references.

## Build/Rebuild APIs

- `UnitBuilder::rebuild_skeleton_egraph()` builds both structures from the current unit.
- `UnitBuilder::finish_rebuild()` is an explicit “commit” call that rebuilds before finishing.
- `UnitBuilderWithRebuild` wraps a builder and rebuilds on `Drop` if no explicit finish occurred.
- `Module::unit_mut_with_rebuild(...)` returns `UnitBuilderWithRebuild` (non-breaking; `unit_mut` is unchanged).

## Debugging

- `Unit::dump_cfg_skeleton()` prints a readable skeleton with block args, effect statements, and terminators.
- `Unit::dump_unit_egraph()` prints the value-to-eclass mapping and tuple count.

## Call-Site Wiring

Explicit rebuilds (via `finish_rebuild`) were added to key unit construction paths:

- `llhd/src/pass/deseq.rs`
- `llhd/src/bin/llhd-conv/liberty.rs`
- `llhd/src/assembly/grammar.rs`

Additionally, Wirelog call sites using mutable unit access now use `unit_mut_with_rebuild`:

- `Wirelog/llhd-egraph/src/llhd/common.rs`
- `Wirelog/wirelog/src/netlist.rs`

## Dependency

- `llhd` now depends on the local `egglog` crate to drive the e-graph integration.

## Notes

- The integration is transient and rebuildable; it does not modify LLHD file formats.
- Rebuild failures log warnings (via `log::warn!`) but do not panic.
