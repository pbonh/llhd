use crate::ir::{
    egglog_schema::{LlhdDfgSchema, VecValue},
    Inst, InstData, Opcode, Unit, Value, ValueData,
};
use crate::table::TableKey;
use crate::ty::{void_ty, Type, TypeKind};
use anyhow::{anyhow, Result};
use egglog_bridge::EGraph;
use egglog_core_relations::Value as BridgeValue;
use std::collections::HashMap;
use std::fmt::{self, Write};

/// A reference to an egglog e-class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EClassRef(pub BridgeValue);

impl fmt::Display for EClassRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// An egglog e-graph backing a single LLHD unit.
#[derive(Clone)]
pub struct UnitEGraph {
    /// Underlying egglog e-graph instance.
    pub egraph: EGraph,
    /// Registered schema tables for LLHD DFG terms.
    pub schema: LlhdDfgSchema,
    /// Map from LLHD values to their e-class representatives.
    pub value_classes: HashMap<Value, EClassRef>,
    value_nodes: HashMap<Value, BridgeValue>,
    ty_nodes: HashMap<Type, BridgeValue>,
}

impl fmt::Debug for UnitEGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitEGraph")
            .field("egraph_tuples", &self.schema.total_table_size(&self.egraph))
            .field("value_classes", &self.value_classes.len())
            .finish()
    }
}

impl Default for UnitEGraph {
    fn default() -> Self {
        let mut egraph = EGraph::default();
        let schema = LlhdDfgSchema::register(&mut egraph);
        Self {
            egraph,
            schema,
            value_classes: HashMap::new(),
            value_nodes: HashMap::new(),
            ty_nodes: HashMap::new(),
        }
    }
}

impl UnitEGraph {
    /// Build an e-graph for a unit and map values to e-classes.
    pub fn build_from_unit(unit: &Unit<'_>) -> Result<Self> {
        let mut egraph = EGraph::default();
        let schema = LlhdDfgSchema::register(&mut egraph);
        let mut out = Self {
            egraph,
            schema,
            value_classes: HashMap::new(),
            value_nodes: HashMap::new(),
            ty_nodes: HashMap::new(),
        };
        out.populate_from_unit(unit)?;
        Ok(out)
    }

    /// Get the e-class for a value, if present.
    pub fn class_for_value(&self, value: Value) -> Option<EClassRef> {
        self.value_classes.get(&value).copied()
    }

    /// Ensure an e-class exists for a value by inserting a ValueRef if needed.
    pub fn ensure_value_ref(&mut self, unit: &Unit<'_>, value: Value) -> Result<EClassRef> {
        if let Some(class) = self.class_for_value(value) {
            return Ok(class);
        }
        let class = EClassRef(self.mk_value_ref(unit, value)?);
        self.value_classes.insert(value, class);
        Ok(class)
    }

    /// Dump the e-graph mapping for debugging.
    pub fn dump(&self, unit: &Unit<'_>) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "egraph tuples: {}",
            self.schema.total_table_size(&self.egraph)
        );
        for (value, class) in self.value_classes.iter() {
            let _ = writeln!(out, "{} => {}", value.dump(unit), class);
        }
        out
    }

    fn populate_from_unit(&mut self, unit: &Unit<'_>) -> Result<()> {
        for value in unit.args() {
            let class = self.ensure_value_ref(unit, value)?;
            self.value_classes.insert(value, class);
        }
        for inst in unit.all_insts() {
            if let Some(value) = unit.get_inst_result(inst) {
                let class = self.build_value_class(unit, value)?;
                self.value_classes.insert(value, class);
            }
        }
        Ok(())
    }

    fn build_value_class(&mut self, unit: &Unit<'_>, value: Value) -> Result<EClassRef> {
        if let Some(class) = self.value_classes.get(&value) {
            return Ok(*class);
        }
        let class = match &unit[value] {
            ValueData::Arg { .. } | ValueData::Placeholder { .. } | ValueData::Invalid => {
                self.ensure_value_ref(unit, value)?
            }
            ValueData::Inst { inst, .. } => {
                let inst_data = &unit[*inst];
                if is_pure_opcode(inst_data.opcode()) {
                    EClassRef(self.build_pure_inst(unit, *inst, inst_data)?)
                } else {
                    self.ensure_value_ref(unit, value)?
                }
            }
        };
        Ok(class)
    }

    fn build_pure_inst(
        &mut self,
        unit: &Unit<'_>,
        inst: Inst,
        data: &InstData,
    ) -> Result<BridgeValue> {
        let inst_id = to_i64(inst.index())?;
        let ty = unit.inst_type(inst);
        let ty_id = self.mk_llhd_ty(&ty)?;
        let opcode = data.opcode();

        let term = match data {
            InstData::ConstInt { imm, .. } => self.mk_const_int(inst_id, ty_id, imm.to_string()),
            InstData::ConstTime { imm, .. } => self.mk_const_time(inst_id, ty_id, imm.to_string()),
            InstData::Array { imms, args, .. } if opcode == Opcode::ArrayUniform => {
                let len = to_i64(imms[0])?;
                let arg = self.build_value_class(unit, args[0])?.0;
                self.mk_array_uniform(inst_id, ty_id, len, arg)
            }
            InstData::Aggregate { args, .. } if opcode == Opcode::Array => {
                let values = self.values_as_llhd_values(unit, args)?;
                self.mk_array(inst_id, values)
            }
            InstData::Aggregate { args, .. } if opcode == Opcode::Struct => {
                let values = self.values_as_llhd_values(unit, args)?;
                self.mk_struct(inst_id, values)
            }
            InstData::Unary { args, .. } => {
                let arg = self.build_value_class(unit, args[0])?.0;
                match opcode {
                    Opcode::Alias => self.mk_alias(inst_id, ty_id, arg),
                    Opcode::Not => self.mk_unary(self.schema.dfg_not, inst_id, ty_id, arg),
                    Opcode::Neg => self.mk_unary(self.schema.dfg_neg, inst_id, ty_id, arg),
                    _ => self.mk_value_ref(unit, unit.inst_result(inst))?,
                }
            }
            InstData::Binary { args, .. } => {
                let lhs = self.build_value_class(unit, args[0])?.0;
                let rhs = self.build_value_class(unit, args[1])?.0;
                match opcode {
                    Opcode::Add => self.mk_binary(self.schema.dfg_add, inst_id, ty_id, lhs, rhs),
                    Opcode::Sub => self.mk_binary(self.schema.dfg_sub, inst_id, ty_id, lhs, rhs),
                    Opcode::And => self.mk_binary(self.schema.dfg_and, inst_id, ty_id, lhs, rhs),
                    Opcode::Or => self.mk_binary(self.schema.dfg_or, inst_id, ty_id, lhs, rhs),
                    Opcode::Xor => self.mk_binary(self.schema.dfg_xor, inst_id, ty_id, lhs, rhs),
                    Opcode::Smul => self.mk_binary(self.schema.dfg_smul, inst_id, ty_id, lhs, rhs),
                    Opcode::Sdiv => self.mk_binary(self.schema.dfg_sdiv, inst_id, ty_id, lhs, rhs),
                    Opcode::Smod => self.mk_binary(self.schema.dfg_smod, inst_id, ty_id, lhs, rhs),
                    Opcode::Srem => self.mk_binary(self.schema.dfg_srem, inst_id, ty_id, lhs, rhs),
                    Opcode::Umul => self.mk_binary(self.schema.dfg_umul, inst_id, ty_id, lhs, rhs),
                    Opcode::Udiv => self.mk_binary(self.schema.dfg_udiv, inst_id, ty_id, lhs, rhs),
                    Opcode::Umod => self.mk_binary(self.schema.dfg_umod, inst_id, ty_id, lhs, rhs),
                    Opcode::Urem => self.mk_binary(self.schema.dfg_urem, inst_id, ty_id, lhs, rhs),
                    Opcode::Eq => self.mk_binary(self.schema.dfg_eq, inst_id, ty_id, lhs, rhs),
                    Opcode::Neq => self.mk_binary(self.schema.dfg_neq, inst_id, ty_id, lhs, rhs),
                    Opcode::Slt => self.mk_binary(self.schema.dfg_slt, inst_id, ty_id, lhs, rhs),
                    Opcode::Sgt => self.mk_binary(self.schema.dfg_sgt, inst_id, ty_id, lhs, rhs),
                    Opcode::Sle => self.mk_binary(self.schema.dfg_sle, inst_id, ty_id, lhs, rhs),
                    Opcode::Sge => self.mk_binary(self.schema.dfg_sge, inst_id, ty_id, lhs, rhs),
                    Opcode::Ult => self.mk_binary(self.schema.dfg_ult, inst_id, ty_id, lhs, rhs),
                    Opcode::Ugt => self.mk_binary(self.schema.dfg_ugt, inst_id, ty_id, lhs, rhs),
                    Opcode::Ule => self.mk_binary(self.schema.dfg_ule, inst_id, ty_id, lhs, rhs),
                    Opcode::Uge => self.mk_binary(self.schema.dfg_uge, inst_id, ty_id, lhs, rhs),
                    Opcode::Mux => self.mk_binary(self.schema.dfg_mux, inst_id, ty_id, lhs, rhs),
                    _ => self.mk_value_ref(unit, unit.inst_result(inst))?,
                }
            }
            InstData::Ternary { args, .. } => {
                let a = self.build_value_class(unit, args[0])?.0;
                let b = self.build_value_class(unit, args[1])?.0;
                let c = self.build_value_class(unit, args[2])?.0;
                match opcode {
                    Opcode::Shl => self.mk_ternary(self.schema.dfg_shl, inst_id, ty_id, a, b, c),
                    Opcode::Shr => self.mk_ternary(self.schema.dfg_shr, inst_id, ty_id, a, b, c),
                    _ => self.mk_value_ref(unit, unit.inst_result(inst))?,
                }
            }
            InstData::InsExt { args, imms, .. }
                if opcode == Opcode::InsField
                    || opcode == Opcode::InsSlice
                    || opcode == Opcode::ExtField
                    || opcode == Opcode::ExtSlice =>
            {
                let a = self.build_value_class(unit, args[0])?.0;
                let b = self.build_value_class(unit, args[1])?.0;
                let imm0 = *imms.get(0).unwrap_or(&0);
                let imm1 = *imms.get(1).unwrap_or(&0);
                let imm0 = to_i64(imm0)?;
                let imm1 = to_i64(imm1)?;
                match opcode {
                    Opcode::InsField => {
                        self.mk_insext(self.schema.dfg_ins_field, inst_id, ty_id, a, b, imm0, imm1)
                    }
                    Opcode::InsSlice => {
                        self.mk_insext(self.schema.dfg_ins_slice, inst_id, ty_id, a, b, imm0, imm1)
                    }
                    Opcode::ExtField => {
                        self.mk_insext(self.schema.dfg_ext_field, inst_id, ty_id, a, b, imm0, imm1)
                    }
                    Opcode::ExtSlice => {
                        self.mk_insext(self.schema.dfg_ext_slice, inst_id, ty_id, a, b, imm0, imm1)
                    }
                    _ => self.mk_value_ref(unit, unit.inst_result(inst))?,
                }
            }
            _ => self.mk_value_ref(unit, unit.inst_result(inst))?,
        };

        Ok(term)
    }

    fn mk_value_ref(&mut self, unit: &Unit<'_>, value: Value) -> Result<BridgeValue> {
        let llhd_value = self.mk_llhd_value(unit, value)?;
        Ok(self.add_term(self.schema.dfg_value_ref, vec![llhd_value]))
    }

    fn mk_llhd_value(&mut self, unit: &Unit<'_>, value: Value) -> Result<BridgeValue> {
        if let Some(node) = self.value_nodes.get(&value) {
            return Ok(*node);
        }
        let ty = match &unit[value] {
            ValueData::Invalid => void_ty(),
            _ => unit.value_type(value),
        };
        let ty_id = self.mk_llhd_ty(&ty)?;
        let id = to_i64(value.index())?;
        let id_value = self.base_i64(id);
        let node = self.add_term(self.schema.value, vec![ty_id, id_value]);
        self.value_nodes.insert(value, node);
        Ok(node)
    }

    fn values_as_llhd_values(
        &mut self,
        unit: &Unit<'_>,
        values: &[Value],
    ) -> Result<Vec<BridgeValue>> {
        values
            .iter()
            .map(|&value| self.mk_llhd_value(unit, value))
            .collect()
    }

    fn mk_llhd_ty(&mut self, ty: &Type) -> Result<BridgeValue> {
        if let Some(node) = self.ty_nodes.get(ty) {
            return Ok(*node);
        }
        let node = match ty.as_ref() {
            TypeKind::VoidType => self.add_term(self.schema.ty_void, vec![]),
            TypeKind::TimeType => self.add_term(self.schema.ty_time, vec![]),
            TypeKind::IntType(bits) => {
                let bits = to_i64(*bits)?;
                let bits_value = self.base_i64(bits);
                self.add_term(self.schema.ty_int, vec![bits_value])
            }
            TypeKind::EnumType(states) => {
                let states = to_i64(*states)?;
                let states_value = self.base_i64(states);
                self.add_term(self.schema.ty_enum, vec![states_value])
            }
            TypeKind::PointerType(inner) => {
                let inner = self.mk_llhd_ty(inner)?;
                self.add_term(self.schema.ty_pointer, vec![inner])
            }
            TypeKind::SignalType(inner) => {
                let inner = self.mk_llhd_ty(inner)?;
                self.add_term(self.schema.ty_signal, vec![inner])
            }
            TypeKind::ArrayType(len, inner) => {
                let len = to_i64(*len)?;
                let inner = self.mk_llhd_ty(inner)?;
                let len_value = self.base_i64(len);
                self.add_term(self.schema.ty_array, vec![len_value, inner])
            }
            TypeKind::StructType(fields) => {
                let elems = fields
                    .iter()
                    .map(|field| self.mk_llhd_ty(field))
                    .collect::<Result<Vec<_>>>()?;
                let vec_value = self.base_vec(elems);
                self.add_term(self.schema.ty_struct, vec![vec_value])
            }
            TypeKind::FuncType(args, ret) => {
                let args = args
                    .iter()
                    .map(|arg| self.mk_llhd_ty(arg))
                    .collect::<Result<Vec<_>>>()?;
                let args = self.base_vec(args);
                let ret = self.mk_llhd_ty(ret)?;
                self.add_term(self.schema.ty_func, vec![args, ret])
            }
            TypeKind::EntityType(ins, outs) => {
                let ins = ins
                    .iter()
                    .map(|arg| self.mk_llhd_ty(arg))
                    .collect::<Result<Vec<_>>>()?;
                let outs = outs
                    .iter()
                    .map(|arg| self.mk_llhd_ty(arg))
                    .collect::<Result<Vec<_>>>()?;
                let ins = self.base_vec(ins);
                let outs = self.base_vec(outs);
                self.add_term(self.schema.ty_entity, vec![ins, outs])
            }
        };
        self.ty_nodes.insert(ty.clone(), node);
        Ok(node)
    }

    fn mk_const_int(&mut self, inst_id: i64, ty_id: BridgeValue, text: String) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        let text_value = self.base_string(text);
        self.add_term(
            self.schema.dfg_const_int,
            vec![inst_value, ty_id, text_value],
        )
    }

    fn mk_const_time(&mut self, inst_id: i64, ty_id: BridgeValue, text: String) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        let text_value = self.base_string(text);
        self.add_term(
            self.schema.dfg_const_time,
            vec![inst_value, ty_id, text_value],
        )
    }

    fn mk_alias(&mut self, inst_id: i64, ty_id: BridgeValue, arg: BridgeValue) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        self.add_term(self.schema.dfg_alias, vec![inst_value, ty_id, arg])
    }

    fn mk_unary(
        &mut self,
        func: egglog_bridge::FunctionId,
        inst_id: i64,
        ty_id: BridgeValue,
        arg: BridgeValue,
    ) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        self.add_term(func, vec![inst_value, ty_id, arg])
    }

    fn mk_binary(
        &mut self,
        func: egglog_bridge::FunctionId,
        inst_id: i64,
        ty_id: BridgeValue,
        lhs: BridgeValue,
        rhs: BridgeValue,
    ) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        self.add_term(func, vec![inst_value, ty_id, lhs, rhs])
    }

    fn mk_ternary(
        &mut self,
        func: egglog_bridge::FunctionId,
        inst_id: i64,
        ty_id: BridgeValue,
        a: BridgeValue,
        b: BridgeValue,
        c: BridgeValue,
    ) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        self.add_term(func, vec![inst_value, ty_id, a, b, c])
    }

    fn mk_insext(
        &mut self,
        func: egglog_bridge::FunctionId,
        inst_id: i64,
        ty_id: BridgeValue,
        a: BridgeValue,
        b: BridgeValue,
        imm0: i64,
        imm1: i64,
    ) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        let imm0_value = self.base_i64(imm0);
        let imm1_value = self.base_i64(imm1);
        self.add_term(func, vec![inst_value, ty_id, a, b, imm0_value, imm1_value])
    }

    fn mk_array_uniform(
        &mut self,
        inst_id: i64,
        ty_id: BridgeValue,
        len: i64,
        arg: BridgeValue,
    ) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        let len_value = self.base_i64(len);
        self.add_term(
            self.schema.dfg_array_uniform,
            vec![inst_value, ty_id, len_value, arg],
        )
    }

    fn mk_array(&mut self, inst_id: i64, values: Vec<BridgeValue>) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        let vec_value = self.base_vec(values);
        self.add_term(self.schema.dfg_array, vec![inst_value, vec_value])
    }

    fn mk_struct(&mut self, inst_id: i64, values: Vec<BridgeValue>) -> BridgeValue {
        let inst_value = self.base_i64(inst_id);
        let vec_value = self.base_vec(values);
        self.add_term(self.schema.dfg_struct, vec![inst_value, vec_value])
    }

    fn add_term(
        &mut self,
        func: egglog_bridge::FunctionId,
        inputs: Vec<BridgeValue>,
    ) -> BridgeValue {
        self.egraph.add_term(func, &inputs, "")
    }

    fn base_i64(&mut self, value: i64) -> BridgeValue {
        self.egraph.base_values().get::<i64>(value)
    }

    fn base_string(&mut self, value: String) -> BridgeValue {
        self.egraph.base_values().get::<String>(value)
    }

    fn base_vec(&mut self, values: Vec<BridgeValue>) -> BridgeValue {
        self.egraph.base_values().get::<VecValue>(VecValue(values))
    }
}

/// Return true for opcodes that are pure in the DFG.
pub fn is_pure_opcode(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::ConstInt
            | Opcode::ConstTime
            | Opcode::Alias
            | Opcode::ArrayUniform
            | Opcode::Array
            | Opcode::Struct
            | Opcode::Not
            | Opcode::Neg
            | Opcode::Add
            | Opcode::Sub
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Smul
            | Opcode::Sdiv
            | Opcode::Smod
            | Opcode::Srem
            | Opcode::Umul
            | Opcode::Udiv
            | Opcode::Umod
            | Opcode::Urem
            | Opcode::Eq
            | Opcode::Neq
            | Opcode::Slt
            | Opcode::Sgt
            | Opcode::Sle
            | Opcode::Sge
            | Opcode::Ult
            | Opcode::Ugt
            | Opcode::Ule
            | Opcode::Uge
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Mux
            | Opcode::InsField
            | Opcode::InsSlice
            | Opcode::ExtField
            | Opcode::ExtSlice
    )
}

fn to_i64(value: usize) -> Result<i64> {
    i64::try_from(value).map_err(|_| anyhow!("value out of i64 range"))
}
