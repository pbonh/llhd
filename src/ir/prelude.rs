// Copyright (c) 2017-2021 Fabian Schuiki

//! Re-exports of commonly used IR items.

#[allow(deprecated)]
pub use crate::ir::{
    cfg_skeleton, Arg, Block, CfgSkeleton, DeclData, DeclId, EGraphLookup, Inst, Module, Opcode,
    RegMode, RegTrigger, Signature, SkBlock, SkEffect, SkTerminator, Unit, UnitBuilder, UnitData,
    UnitId, UnitKind, UnitName, Value,
};
