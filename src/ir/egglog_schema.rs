use egglog_bridge::{ColumnTy, DefaultVal, EGraph, FunctionConfig, FunctionId, MergeFn};
use egglog_core_relations::BaseValue;
use egglog_core_relations::Value;

#[cfg(test)]
use std::fmt::Write;

/// Wrapper for Vec values stored in egglog base tables.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VecValue(pub Vec<Value>);

impl BaseValue for VecValue {}

/// Egglog table ids for the LLHD DFG schema.
#[derive(Debug, Clone)]
pub struct LlhdDfgSchema {
    /// Table id for the void type.
    pub ty_void: FunctionId,
    /// Table id for the time type.
    pub ty_time: FunctionId,
    /// Table id for the integer type.
    pub ty_int: FunctionId,
    /// Table id for the enum type.
    pub ty_enum: FunctionId,
    /// Table id for the pointer type.
    pub ty_pointer: FunctionId,
    /// Table id for the signal type.
    pub ty_signal: FunctionId,
    /// Table id for the array type.
    pub ty_array: FunctionId,
    /// Table id for the struct type.
    pub ty_struct: FunctionId,
    /// Table id for the function type.
    pub ty_func: FunctionId,
    /// Table id for the entity type.
    pub ty_entity: FunctionId,
    /// Table id for the entity unit kind.
    pub unit_kind_entity: FunctionId,
    /// Table id for the function unit kind.
    pub unit_kind_function: FunctionId,
    /// Table id for the process unit kind.
    pub unit_kind_process: FunctionId,
    /// Table id for value nodes.
    pub value: FunctionId,
    /// Table id for basic blocks.
    pub block: FunctionId,
    /// Table id for external unit references.
    pub ext_unit: FunctionId,
    /// Table id for time value literals.
    pub time_value: FunctionId,
    /// Table id for low register mode.
    pub regmode_low: FunctionId,
    /// Table id for high register mode.
    pub regmode_high: FunctionId,
    /// Table id for rising-edge register mode.
    pub regmode_rise: FunctionId,
    /// Table id for falling-edge register mode.
    pub regmode_fall: FunctionId,
    /// Table id for both-edge register mode.
    pub regmode_both: FunctionId,
    /// Table id for DFG ValueRef nodes.
    pub dfg_value_ref: FunctionId,
    /// Table id for DFG ConstInt nodes.
    pub dfg_const_int: FunctionId,
    /// Table id for DFG ConstTime nodes.
    pub dfg_const_time: FunctionId,
    /// Table id for DFG Alias nodes.
    pub dfg_alias: FunctionId,
    /// Table id for DFG ArrayUniform nodes.
    pub dfg_array_uniform: FunctionId,
    /// Table id for DFG Array nodes.
    pub dfg_array: FunctionId,
    /// Table id for DFG Struct nodes.
    pub dfg_struct: FunctionId,
    /// Table id for DFG Not nodes.
    pub dfg_not: FunctionId,
    /// Table id for DFG Neg nodes.
    pub dfg_neg: FunctionId,
    /// Table id for DFG Add nodes.
    pub dfg_add: FunctionId,
    /// Table id for DFG Sub nodes.
    pub dfg_sub: FunctionId,
    /// Table id for DFG And nodes.
    pub dfg_and: FunctionId,
    /// Table id for DFG Or nodes.
    pub dfg_or: FunctionId,
    /// Table id for DFG Xor nodes.
    pub dfg_xor: FunctionId,
    /// Table id for DFG Smul nodes.
    pub dfg_smul: FunctionId,
    /// Table id for DFG Sdiv nodes.
    pub dfg_sdiv: FunctionId,
    /// Table id for DFG Smod nodes.
    pub dfg_smod: FunctionId,
    /// Table id for DFG Srem nodes.
    pub dfg_srem: FunctionId,
    /// Table id for DFG Umul nodes.
    pub dfg_umul: FunctionId,
    /// Table id for DFG Udiv nodes.
    pub dfg_udiv: FunctionId,
    /// Table id for DFG Umod nodes.
    pub dfg_umod: FunctionId,
    /// Table id for DFG Urem nodes.
    pub dfg_urem: FunctionId,
    /// Table id for DFG Eq nodes.
    pub dfg_eq: FunctionId,
    /// Table id for DFG Neq nodes.
    pub dfg_neq: FunctionId,
    /// Table id for DFG Slt nodes.
    pub dfg_slt: FunctionId,
    /// Table id for DFG Sgt nodes.
    pub dfg_sgt: FunctionId,
    /// Table id for DFG Sle nodes.
    pub dfg_sle: FunctionId,
    /// Table id for DFG Sge nodes.
    pub dfg_sge: FunctionId,
    /// Table id for DFG Ult nodes.
    pub dfg_ult: FunctionId,
    /// Table id for DFG Ugt nodes.
    pub dfg_ugt: FunctionId,
    /// Table id for DFG Ule nodes.
    pub dfg_ule: FunctionId,
    /// Table id for DFG Uge nodes.
    pub dfg_uge: FunctionId,
    /// Table id for DFG Shl nodes.
    pub dfg_shl: FunctionId,
    /// Table id for DFG Shr nodes.
    pub dfg_shr: FunctionId,
    /// Table id for DFG Mux nodes.
    pub dfg_mux: FunctionId,
    /// Table id for DFG Reg nodes.
    pub dfg_reg: FunctionId,
    /// Table id for DFG InsField nodes.
    pub dfg_ins_field: FunctionId,
    /// Table id for DFG InsSlice nodes.
    pub dfg_ins_slice: FunctionId,
    /// Table id for DFG ExtField nodes.
    pub dfg_ext_field: FunctionId,
    /// Table id for DFG ExtSlice nodes.
    pub dfg_ext_slice: FunctionId,
    /// Table id for DFG Con nodes.
    pub dfg_con: FunctionId,
    /// Table id for DFG Del nodes.
    pub dfg_del: FunctionId,
    /// Table id for DFG Call nodes.
    pub dfg_call: FunctionId,
    /// Table id for DFG Inst nodes.
    pub dfg_inst: FunctionId,
    /// Table id for DFG Sig nodes.
    pub dfg_sig: FunctionId,
    /// Table id for DFG Prb nodes.
    pub dfg_prb: FunctionId,
    /// Table id for DFG Drv nodes.
    pub dfg_drv: FunctionId,
    /// Table id for DFG DrvCond nodes.
    pub dfg_drv_cond: FunctionId,
    /// Table id for DFG Var nodes.
    pub dfg_var: FunctionId,
    /// Table id for DFG Ld nodes.
    pub dfg_ld: FunctionId,
    /// Table id for DFG St nodes.
    pub dfg_st: FunctionId,
    /// Table id for DFG Halt nodes.
    pub dfg_halt: FunctionId,
    /// Table id for DFG Ret nodes.
    pub dfg_ret: FunctionId,
    /// Table id for DFG RetValue nodes.
    pub dfg_ret_value: FunctionId,
    /// Table id for DFG Phi nodes.
    pub dfg_phi: FunctionId,
    /// Table id for DFG Br nodes.
    pub dfg_br: FunctionId,
    /// Table id for DFG BrCond nodes.
    pub dfg_br_cond: FunctionId,
    /// Table id for DFG Wait nodes.
    pub dfg_wait: FunctionId,
    /// Table id for DFG WaitTime nodes.
    pub dfg_wait_time: FunctionId,
}

impl LlhdDfgSchema {
    /// Register all LLHD DFG tables in the e-graph.
    pub fn register(egraph: &mut EGraph) -> Self {
        let int_base = egraph.base_values_mut().register_type::<i64>();
        let string_base = egraph.base_values_mut().register_type::<String>();
        let vec_base = egraph.base_values_mut().register_type::<VecValue>();

        let ctor = |egraph: &mut EGraph, name: &str, schema: Vec<ColumnTy>| {
            egraph.add_table(FunctionConfig {
                schema,
                default: DefaultVal::FreshId,
                merge: MergeFn::UnionId,
                name: name.into(),
                can_subsume: false,
            })
        };

        Self {
            ty_void: ctor(egraph, "Void", vec![ColumnTy::Id]),
            ty_time: ctor(egraph, "Time", vec![ColumnTy::Id]),
            ty_int: ctor(
                egraph,
                "IntTy",
                vec![ColumnTy::Base(int_base), ColumnTy::Id],
            ),
            ty_enum: ctor(egraph, "Enum", vec![ColumnTy::Base(int_base), ColumnTy::Id]),
            ty_pointer: ctor(egraph, "Pointer", vec![ColumnTy::Id, ColumnTy::Id]),
            ty_signal: ctor(egraph, "Signal", vec![ColumnTy::Id, ColumnTy::Id]),
            ty_array: ctor(
                egraph,
                "ArrayTy",
                vec![ColumnTy::Base(int_base), ColumnTy::Id, ColumnTy::Id],
            ),
            ty_struct: ctor(
                egraph,
                "StructTy",
                vec![ColumnTy::Base(vec_base), ColumnTy::Id],
            ),
            ty_func: ctor(
                egraph,
                "FuncTy",
                vec![ColumnTy::Base(vec_base), ColumnTy::Id, ColumnTy::Id],
            ),
            ty_entity: ctor(
                egraph,
                "EntityTy",
                vec![
                    ColumnTy::Base(vec_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            unit_kind_entity: ctor(egraph, "Entity", vec![ColumnTy::Id]),
            unit_kind_function: ctor(egraph, "Function", vec![ColumnTy::Id]),
            unit_kind_process: ctor(egraph, "Process", vec![ColumnTy::Id]),
            value: ctor(
                egraph,
                "Value",
                vec![ColumnTy::Id, ColumnTy::Base(int_base), ColumnTy::Id],
            ),
            block: ctor(
                egraph,
                "Block",
                vec![ColumnTy::Base(int_base), ColumnTy::Id],
            ),
            ext_unit: ctor(
                egraph,
                "ExtUnit",
                vec![ColumnTy::Base(int_base), ColumnTy::Id],
            ),
            time_value: ctor(
                egraph,
                "TimeValue",
                vec![ColumnTy::Base(int_base), ColumnTy::Id],
            ),
            regmode_low: ctor(egraph, "Low", vec![ColumnTy::Id]),
            regmode_high: ctor(egraph, "High", vec![ColumnTy::Id]),
            regmode_rise: ctor(egraph, "Rise", vec![ColumnTy::Id]),
            regmode_fall: ctor(egraph, "Fall", vec![ColumnTy::Id]),
            regmode_both: ctor(egraph, "Both", vec![ColumnTy::Id]),
            dfg_value_ref: ctor(egraph, "ValueRef", vec![ColumnTy::Id, ColumnTy::Id]),
            dfg_const_int: ctor(
                egraph,
                "ConstInt",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Base(string_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_const_time: ctor(
                egraph,
                "ConstTime",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Base(string_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_alias: ctor(
                egraph,
                "Alias",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_array_uniform: ctor(
                egraph,
                "ArrayUniform",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_array: ctor(
                egraph,
                "Array",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_struct: ctor(
                egraph,
                "Struct",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_not: ctor(
                egraph,
                "Not",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_neg: ctor(
                egraph,
                "Neg",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_add: ctor(
                egraph,
                "Add",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_sub: ctor(
                egraph,
                "Sub",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_and: ctor(
                egraph,
                "And",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_or: ctor(
                egraph,
                "Or",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_xor: ctor(
                egraph,
                "Xor",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_smul: ctor(
                egraph,
                "Smul",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_sdiv: ctor(
                egraph,
                "Sdiv",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_smod: ctor(
                egraph,
                "Smod",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_srem: ctor(
                egraph,
                "Srem",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_umul: ctor(
                egraph,
                "Umul",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_udiv: ctor(
                egraph,
                "Udiv",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_umod: ctor(
                egraph,
                "Umod",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_urem: ctor(
                egraph,
                "Urem",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_eq: ctor(
                egraph,
                "Eq",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_neq: ctor(
                egraph,
                "Neq",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_slt: ctor(
                egraph,
                "Slt",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_sgt: ctor(
                egraph,
                "Sgt",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_sle: ctor(
                egraph,
                "Sle",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_sge: ctor(
                egraph,
                "Sge",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_ult: ctor(
                egraph,
                "Ult",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_ugt: ctor(
                egraph,
                "Ugt",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_ule: ctor(
                egraph,
                "Ule",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_uge: ctor(
                egraph,
                "Uge",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_shl: ctor(
                egraph,
                "Shl",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_shr: ctor(
                egraph,
                "Shr",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_mux: ctor(
                egraph,
                "Mux",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_reg: ctor(
                egraph,
                "Reg",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Base(vec_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_ins_field: ctor(
                egraph,
                "InsField",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_ins_slice: ctor(
                egraph,
                "InsSlice",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_ext_field: ctor(
                egraph,
                "ExtField",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_ext_slice: ctor(
                egraph,
                "ExtSlice",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_con: ctor(
                egraph,
                "Con",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_del: ctor(
                egraph,
                "Del",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_call: ctor(
                egraph,
                "Call",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_inst: ctor(
                egraph,
                "Inst",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_sig: ctor(
                egraph,
                "Sig",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_prb: ctor(
                egraph,
                "Prb",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_drv: ctor(
                egraph,
                "Drv",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_drv_cond: ctor(
                egraph,
                "DrvCond",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_var: ctor(
                egraph,
                "Var",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_ld: ctor(
                egraph,
                "Ld",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_st: ctor(
                egraph,
                "St",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_halt: ctor(egraph, "Halt", vec![ColumnTy::Base(int_base), ColumnTy::Id]),
            dfg_ret: ctor(egraph, "Ret", vec![ColumnTy::Base(int_base), ColumnTy::Id]),
            dfg_ret_value: ctor(
                egraph,
                "RetValue",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_phi: ctor(
                egraph,
                "Phi",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_br: ctor(
                egraph,
                "Br",
                vec![ColumnTy::Base(int_base), ColumnTy::Id, ColumnTy::Id],
            ),
            dfg_br_cond: ctor(
                egraph,
                "BrCond",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                    ColumnTy::Id,
                ],
            ),
            dfg_wait: ctor(
                egraph,
                "Wait",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
            dfg_wait_time: ctor(
                egraph,
                "WaitTime",
                vec![
                    ColumnTy::Base(int_base),
                    ColumnTy::Id,
                    ColumnTy::Base(vec_base),
                    ColumnTy::Id,
                ],
            ),
        }
    }

    /// Sum the row counts across all registered tables.
    pub fn total_table_size(&self, egraph: &EGraph) -> usize {
        let tables = [
            self.ty_void,
            self.ty_time,
            self.ty_int,
            self.ty_enum,
            self.ty_pointer,
            self.ty_signal,
            self.ty_array,
            self.ty_struct,
            self.ty_func,
            self.ty_entity,
            self.unit_kind_entity,
            self.unit_kind_function,
            self.unit_kind_process,
            self.value,
            self.block,
            self.ext_unit,
            self.time_value,
            self.regmode_low,
            self.regmode_high,
            self.regmode_rise,
            self.regmode_fall,
            self.regmode_both,
            self.dfg_value_ref,
            self.dfg_const_int,
            self.dfg_const_time,
            self.dfg_alias,
            self.dfg_array_uniform,
            self.dfg_array,
            self.dfg_struct,
            self.dfg_not,
            self.dfg_neg,
            self.dfg_add,
            self.dfg_sub,
            self.dfg_and,
            self.dfg_or,
            self.dfg_xor,
            self.dfg_smul,
            self.dfg_sdiv,
            self.dfg_smod,
            self.dfg_srem,
            self.dfg_umul,
            self.dfg_udiv,
            self.dfg_umod,
            self.dfg_urem,
            self.dfg_eq,
            self.dfg_neq,
            self.dfg_slt,
            self.dfg_sgt,
            self.dfg_sle,
            self.dfg_sge,
            self.dfg_ult,
            self.dfg_ugt,
            self.dfg_ule,
            self.dfg_uge,
            self.dfg_shl,
            self.dfg_shr,
            self.dfg_mux,
            self.dfg_reg,
            self.dfg_ins_field,
            self.dfg_ins_slice,
            self.dfg_ext_field,
            self.dfg_ext_slice,
            self.dfg_con,
            self.dfg_del,
            self.dfg_call,
            self.dfg_inst,
            self.dfg_sig,
            self.dfg_prb,
            self.dfg_drv,
            self.dfg_drv_cond,
            self.dfg_var,
            self.dfg_ld,
            self.dfg_st,
            self.dfg_halt,
            self.dfg_ret,
            self.dfg_ret_value,
            self.dfg_phi,
            self.dfg_br,
            self.dfg_br_cond,
            self.dfg_wait,
            self.dfg_wait_time,
        ];
        tables.iter().map(|table| egraph.table_size(*table)).sum()
    }
}

/// Egglog table ids for the LLHD CFG skeleton schema.
#[derive(Debug, Clone)]
pub struct LlhdCfgSchema {
    /// Table id for CFG skeleton blocks.
    pub sk_block: FunctionId,
    /// Table id for CFG skeleton block arguments.
    pub sk_block_arg: FunctionId,
    /// Table id for CFG skeleton effectful statements.
    pub sk_effect: FunctionId,
    /// Table id for CFG skeleton branch terminators.
    pub sk_term_br: FunctionId,
    /// Table id for CFG skeleton conditional branch terminators.
    pub sk_term_br_cond: FunctionId,
    /// Table id for CFG skeleton wait terminators.
    pub sk_term_wait: FunctionId,
    /// Table id for CFG skeleton waittime terminators.
    pub sk_term_wait_time: FunctionId,
    /// Table id for CFG skeleton return terminators.
    pub sk_term_ret: FunctionId,
    /// Table id for CFG skeleton return-value terminators.
    pub sk_term_ret_value: FunctionId,
    /// Table id for CFG skeleton halt terminators.
    pub sk_term_halt: FunctionId,
}

impl LlhdCfgSchema {
    /// Register all LLHD CFG skeleton tables in the e-graph.
    pub fn register(egraph: &mut EGraph) -> Self {
        let int_base = egraph.base_values_mut().register_type::<i64>();
        let string_base = egraph.base_values_mut().register_type::<String>();
        let vec_base = egraph.base_values_mut().register_type::<VecValue>();

        let ctor = |egraph: &mut EGraph, name: &str, schema: Vec<ColumnTy>| {
            egraph.add_table(FunctionConfig {
                schema,
                default: DefaultVal::FreshId,
                merge: MergeFn::UnionId,
                name: name.into(),
                can_subsume: false,
            })
        };

        let table = |egraph: &mut EGraph, table: &CfgTable| {
            let schema = table
                .columns
                .iter()
                .map(|column| match column.ty {
                    CfgColumnTy::Id => ColumnTy::Id,
                    CfgColumnTy::Int => ColumnTy::Base(int_base),
                    CfgColumnTy::Atom => ColumnTy::Base(string_base),
                    CfgColumnTy::VecValue => ColumnTy::Base(vec_base),
                })
                .collect();
            ctor(egraph, table.name, schema)
        };

        Self {
            sk_block: table(egraph, &CFG_TABLES[0]),
            sk_block_arg: table(egraph, &CFG_TABLES[1]),
            sk_effect: table(egraph, &CFG_TABLES[2]),
            sk_term_br: table(egraph, &CFG_TABLES[3]),
            sk_term_br_cond: table(egraph, &CFG_TABLES[4]),
            sk_term_wait: table(egraph, &CFG_TABLES[5]),
            sk_term_wait_time: table(egraph, &CFG_TABLES[6]),
            sk_term_ret: table(egraph, &CFG_TABLES[7]),
            sk_term_ret_value: table(egraph, &CFG_TABLES[8]),
            sk_term_halt: table(egraph, &CFG_TABLES[9]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CfgColumnTy {
    Id,
    Int,
    Atom,
    VecValue,
}

impl CfgColumnTy {
    #[cfg(test)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Id => "Id",
            Self::Int => "i64",
            Self::Atom => "Atom",
            Self::VecValue => "VecValue",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CfgColumn {
    name: &'static str,
    ty: CfgColumnTy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CfgTable {
    name: &'static str,
    columns: &'static [CfgColumn],
}

/// Table name for CFG skeleton blocks.
pub const CFG_SK_BLOCK: &str = "SkBlock";
/// Table name for CFG skeleton block arguments.
pub const CFG_SK_BLOCK_ARG: &str = "SkBlockArg";
/// Table name for CFG skeleton effectful statements.
pub const CFG_SK_EFFECT: &str = "SkEffect";
/// Table name for CFG skeleton branch terminators.
pub const CFG_SK_TERM_BR: &str = "SkTermBr";
/// Table name for CFG skeleton conditional branch terminators.
pub const CFG_SK_TERM_BR_COND: &str = "SkTermBrCond";
/// Table name for CFG skeleton wait terminators.
pub const CFG_SK_TERM_WAIT: &str = "SkTermWait";
/// Table name for CFG skeleton waittime terminators.
pub const CFG_SK_TERM_WAIT_TIME: &str = "SkTermWaitTime";
/// Table name for CFG skeleton return terminators.
pub const CFG_SK_TERM_RET: &str = "SkTermRet";
/// Table name for CFG skeleton return-value terminators.
pub const CFG_SK_TERM_RET_VALUE: &str = "SkTermRetValue";
/// Table name for CFG skeleton halt terminators.
pub const CFG_SK_TERM_HALT: &str = "SkTermHalt";

const CFG_SK_BLOCK_COLUMNS: &[CfgColumn] = &[CfgColumn {
    name: "block",
    ty: CfgColumnTy::Int,
}];

const CFG_SK_BLOCK_ARG_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "value",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "class",
        ty: CfgColumnTy::Id,
    },
];

const CFG_SK_EFFECT_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "opcode",
        ty: CfgColumnTy::Atom,
    },
    CfgColumn {
        name: "result",
        ty: CfgColumnTy::Id,
    },
    CfgColumn {
        name: "args",
        ty: CfgColumnTy::VecValue,
    },
];

const CFG_SK_TERM_BR_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "target",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "args",
        ty: CfgColumnTy::VecValue,
    },
];

const CFG_SK_TERM_BR_COND_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "cond",
        ty: CfgColumnTy::Id,
    },
    CfgColumn {
        name: "then_target",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "then_args",
        ty: CfgColumnTy::VecValue,
    },
    CfgColumn {
        name: "else_target",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "else_args",
        ty: CfgColumnTy::VecValue,
    },
];

const CFG_SK_TERM_WAIT_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "target",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "args",
        ty: CfgColumnTy::VecValue,
    },
];

const CFG_SK_TERM_WAIT_TIME_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "time",
        ty: CfgColumnTy::Id,
    },
    CfgColumn {
        name: "target",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "args",
        ty: CfgColumnTy::VecValue,
    },
];

const CFG_SK_TERM_RET_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
];

const CFG_SK_TERM_RET_VALUE_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "value",
        ty: CfgColumnTy::Id,
    },
];

const CFG_SK_TERM_HALT_COLUMNS: &[CfgColumn] = &[
    CfgColumn {
        name: "block",
        ty: CfgColumnTy::Int,
    },
    CfgColumn {
        name: "inst",
        ty: CfgColumnTy::Int,
    },
];

const CFG_TABLES: &[CfgTable] = &[
    CfgTable {
        name: CFG_SK_BLOCK,
        columns: CFG_SK_BLOCK_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_BLOCK_ARG,
        columns: CFG_SK_BLOCK_ARG_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_EFFECT,
        columns: CFG_SK_EFFECT_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_TERM_BR,
        columns: CFG_SK_TERM_BR_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_TERM_BR_COND,
        columns: CFG_SK_TERM_BR_COND_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_TERM_WAIT,
        columns: CFG_SK_TERM_WAIT_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_TERM_WAIT_TIME,
        columns: CFG_SK_TERM_WAIT_TIME_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_TERM_RET,
        columns: CFG_SK_TERM_RET_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_TERM_RET_VALUE,
        columns: CFG_SK_TERM_RET_VALUE_COLUMNS,
    },
    CfgTable {
        name: CFG_SK_TERM_HALT,
        columns: CFG_SK_TERM_HALT_COLUMNS,
    },
];

/// Reference CFG schema as a readable egglog program string.
pub fn cfg_schema_program() -> &'static str {
    CFG_SCHEMA_PROGRAM
}

const CFG_SCHEMA_PROGRAM: &str = concat!(
    "(table SkBlock (block i64))\n",
    "(table SkBlockArg (block i64) (value i64) (class Id))\n",
    "(table SkEffect (block i64) (inst i64) (opcode Atom) (result Id) (args VecValue))\n",
    "(table SkTermBr (block i64) (inst i64) (target i64) (args VecValue))\n",
    "(table SkTermBrCond (block i64) (inst i64) (cond Id) (then_target i64) (then_args VecValue) (else_target i64) (else_args VecValue))\n",
    "(table SkTermWait (block i64) (inst i64) (target i64) (args VecValue))\n",
    "(table SkTermWaitTime (block i64) (inst i64) (time Id) (target i64) (args VecValue))\n",
    "(table SkTermRet (block i64) (inst i64))\n",
    "(table SkTermRetValue (block i64) (inst i64) (value Id))\n",
    "(table SkTermHalt (block i64) (inst i64))\n",
);

#[cfg(test)]
fn render_cfg_schema_program() -> String {
    let mut out = String::new();
    for table in CFG_TABLES {
        let _ = write!(out, "(table {}", table.name);
        for column in table.columns {
            let _ = write!(out, " ({} {})", column.name, column.ty.as_str());
        }
        out.push_str(")\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_schema_matches_reference() {
        assert_eq!(render_cfg_schema_program(), cfg_schema_program());
    }
}
