use crate::ir::*;

/// LLHD Scope, which defines ownership semantics for every entity created inside of an LLHD Module
#[derive(
    Debug, Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum LLHDScope {
    /// Entire Module
    #[default]
    Module,
    /// Specific Unit
    Unit(UnitId),
    /// Unit Arg
    ValueDef(UnitId, Value),
    /// Unit Inst
    Inst(UnitId, Value, Inst),
    /// Inst Value Reference
    ValueRef(UnitId, Value, Inst, Value),
}

impl From<()> for LLHDScope {
    fn from(_no_scope: ()) -> Self {
        Self::Module
    }
}

impl From<UnitId> for LLHDScope {
    fn from(unit_id: UnitId) -> Self {
        Self::Unit(unit_id)
    }
}

impl From<(UnitId, Value)> for LLHDScope {
    fn from(value_def: (UnitId, Value)) -> Self {
        Self::ValueDef(value_def.0, value_def.1)
    }
}

impl From<(UnitId, Value, Inst)> for LLHDScope {
    fn from(inst_def: (UnitId, Value, Inst)) -> Self {
        Self::Inst(inst_def.0, inst_def.1, inst_def.2)
    }
}

impl From<(UnitId, Value, Inst, Value)> for LLHDScope {
    fn from(value_ref: (UnitId, Value, Inst, Value)) -> Self {
        Self::ValueRef(value_ref.0, value_ref.1, value_ref.2, value_ref.3)
    }
}
