// Copyright (c) 2017-2021 Fabian Schuiki

//! Re-exports of commonly used IR items.

#[allow(deprecated)]
pub use crate::ir::{
    Arg, Block, CfgSkeleton, DeclData, DeclId, EClassRef, Inst, Module, Opcode, RegMode,
    RegTrigger, Signature, Unit, UnitBuilder, UnitBuilderWithRebuild, UnitData, UnitEGraph, UnitId,
    UnitKind, UnitName, Value,
};
