use crate::ir::{Inst, InstData, Opcode, Unit, Value, ValueData};
use crate::table::TableKey;
use crate::ty::{void_ty, Type, TypeKind};
use egglog::{EGraph, Error, Value as EggValue};
use std::collections::HashMap;
use std::fmt::{self, Write};

const LLHD_DFG_SORTS: &str = include_str!("../../../Wirelog/resources/egglog/llhd_dfg_sort.egg");

/// A reference to an egglog e-class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EClassRef(pub EggValue);

impl fmt::Display for EClassRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// An egglog e-graph backing a single LLHD unit.
#[derive(Clone, Default)]
pub struct UnitEGraph {
    /// The underlying egglog e-graph.
    pub egraph: EGraph,
    /// Value to e-class mapping for the unit.
    pub value_classes: HashMap<Value, EClassRef>,
}

impl fmt::Debug for UnitEGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitEGraph")
            .field("egraph_tuples", &self.egraph.num_tuples())
            .field("value_classes", &self.value_classes.len())
            .finish()
    }
}

impl UnitEGraph {
    /// Build an e-graph for a unit and map values to e-classes.
    pub fn build_from_unit(unit: &Unit<'_>) -> Result<Self, Error> {
        let mut egraph = EGraph::default();
        egraph.parse_and_run_program(None, LLHD_DFG_SORTS)?;
        let mut out = Self {
            egraph,
            value_classes: HashMap::new(),
        };
        out.populate_from_unit(unit)?;
        Ok(out)
    }

    /// Get the e-class for a value, if present.
    pub fn class_for_value(&self, value: Value) -> Option<EClassRef> {
        self.value_classes.get(&value).copied()
    }

    /// Ensure an e-class exists for a value by inserting a ValueRef if needed.
    pub fn ensure_value_ref(&mut self, unit: &Unit<'_>, value: Value) -> Result<EClassRef, Error> {
        if let Some(class) = self.class_for_value(value) {
            return Ok(class);
        }
        let expr = value_ref_expr(unit, value);
        let class = self.eval_expr_str(&expr)?;
        self.value_classes.insert(value, class);
        Ok(class)
    }

    /// Dump the e-graph mapping for debugging.
    pub fn dump(&self, unit: &Unit<'_>) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "egraph tuples: {}", self.egraph.num_tuples());
        for (value, class) in self.value_classes.iter() {
            let _ = writeln!(out, "{} => {}", value.dump(unit), class);
        }
        out
    }

    fn populate_from_unit(&mut self, unit: &Unit<'_>) -> Result<(), Error> {
        let mut expr_cache: HashMap<Value, String> = HashMap::new();
        for value in unit.args() {
            let expr = value_ref_expr(unit, value);
            let class = self.eval_expr_str(&expr)?;
            self.value_classes.insert(value, class);
        }
        for inst in unit.all_insts() {
            if let Some(value) = unit.get_inst_result(inst) {
                let expr = dfg_expr_for_value(unit, value, &mut expr_cache);
                let class = self.eval_expr_str(&expr)?;
                self.value_classes.insert(value, class);
            }
        }
        Ok(())
    }

    fn eval_expr_str(&mut self, expr: &str) -> Result<EClassRef, Error> {
        let expr = self.egraph.parser.get_expr_from_string(None, expr)?;
        let (_sort, value) = self.egraph.eval_expr(&expr)?;
        Ok(EClassRef(value))
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

fn dfg_expr_for_value(unit: &Unit<'_>, value: Value, cache: &mut HashMap<Value, String>) -> String {
    if let Some(expr) = cache.get(&value) {
        return expr.clone();
    }
    let expr = match unit[value].clone() {
        ValueData::Arg { .. } => value_ref_expr(unit, value),
        ValueData::Placeholder { .. } => value_ref_expr(unit, value),
        ValueData::Invalid => value_ref_expr_with_type(void_ty(), value),
        ValueData::Inst { inst, .. } => {
            let inst_data = &unit[inst];
            if is_pure_opcode(inst_data.opcode()) {
                inst_expr(unit, inst, inst_data, cache)
            } else {
                value_ref_expr(unit, value)
            }
        }
    };
    cache.insert(value, expr.clone());
    expr
}

fn inst_expr(
    unit: &Unit<'_>,
    inst: Inst,
    inst_data: &InstData,
    cache: &mut HashMap<Value, String>,
) -> String {
    let inst_id = format_i64(inst.index());
    let ty_expr = type_expr(&unit.inst_type(inst));
    let opcode = inst_data.opcode();

    match inst_data {
        InstData::ConstInt { imm, .. } => format!(
            "({} {} {} \"{}\")",
            opcode_symbol(opcode),
            inst_id,
            ty_expr,
            escape_string(&imm.to_string())
        ),
        InstData::ConstTime { imm, .. } => format!(
            "({} {} {} \"{}\")",
            opcode_symbol(opcode),
            inst_id,
            ty_expr,
            escape_string(&imm.to_string())
        ),
        InstData::Array { imms, args, .. } if opcode == Opcode::ArrayUniform => format!(
            "({} {} {} {} {})",
            opcode_symbol(opcode),
            inst_id,
            ty_expr,
            format_i64(imms[0]),
            dfg_expr_for_value(unit, args[0], cache)
        ),
        InstData::Aggregate { args, .. } if opcode == Opcode::Array || opcode == Opcode::Struct => {
            let values = args
                .iter()
                .map(|&arg| value_expr(unit, arg))
                .collect::<Vec<_>>();
            format!(
                "({} {} {})",
                opcode_symbol(opcode),
                inst_id,
                vec_expr(&values)
            )
        }
        InstData::Unary { args, .. } => format!(
            "({} {} {} {})",
            opcode_symbol(opcode),
            inst_id,
            ty_expr,
            dfg_expr_for_value(unit, args[0], cache)
        ),
        InstData::Binary { args, .. } => format!(
            "({} {} {} {} {})",
            opcode_symbol(opcode),
            inst_id,
            ty_expr,
            dfg_expr_for_value(unit, args[0], cache),
            dfg_expr_for_value(unit, args[1], cache)
        ),
        InstData::Ternary { args, .. } => format!(
            "({} {} {} {} {} {})",
            opcode_symbol(opcode),
            inst_id,
            ty_expr,
            dfg_expr_for_value(unit, args[0], cache),
            dfg_expr_for_value(unit, args[1], cache),
            dfg_expr_for_value(unit, args[2], cache)
        ),
        InstData::InsExt { args, imms, .. }
            if opcode == Opcode::InsField
                || opcode == Opcode::InsSlice
                || opcode == Opcode::ExtField
                || opcode == Opcode::ExtSlice =>
        {
            let imm0 = *imms.get(0).unwrap_or(&0);
            let imm1 = *imms.get(1).unwrap_or(&0);
            format!(
                "({} {} {} {} {} {} {})",
                opcode_symbol(opcode),
                inst_id,
                ty_expr,
                dfg_expr_for_value(unit, args[0], cache),
                dfg_expr_for_value(unit, args[1], cache),
                format_i64(imm0),
                format_i64(imm1)
            )
        }
        _ => value_ref_expr(unit, unit.inst_result(inst)),
    }
}

fn value_ref_expr(unit: &Unit<'_>, value: Value) -> String {
    let ty = match unit[value] {
        ValueData::Invalid => void_ty(),
        _ => unit.value_type(value),
    };
    value_ref_expr_with_type(ty, value)
}

fn value_ref_expr_with_type(ty: Type, value: Value) -> String {
    format!("(ValueRef {})", value_expr_with_type(&ty, value))
}

fn value_expr(unit: &Unit<'_>, value: Value) -> String {
    let ty = unit.value_type(value);
    value_expr_with_type(&ty, value)
}

fn value_expr_with_type(ty: &Type, value: Value) -> String {
    format!("(Value {} {})", type_expr(ty), format_i64(value.index()))
}

fn type_expr(ty: &Type) -> String {
    match ty.as_ref() {
        TypeKind::VoidType => "(Void )".to_string(),
        TypeKind::TimeType => "(Time )".to_string(),
        TypeKind::IntType(bits) => format!("(IntTy {})", format_i64(*bits)),
        TypeKind::EnumType(states) => format!("(Enum {})", format_i64(*states)),
        TypeKind::PointerType(inner) => format!("(Pointer {})", type_expr(inner)),
        TypeKind::SignalType(inner) => format!("(Signal {})", type_expr(inner)),
        TypeKind::ArrayType(len, inner) => {
            format!("(ArrayTy {} {})", format_i64(*len), type_expr(inner))
        }
        TypeKind::StructType(fields) => {
            let elems = fields.iter().map(type_expr).collect::<Vec<_>>();
            format!("(StructTy {})", vec_expr(&elems))
        }
        TypeKind::FuncType(args, ret) => {
            let elems = args.iter().map(type_expr).collect::<Vec<_>>();
            format!("(FuncTy {} {})", vec_expr(&elems), type_expr(ret))
        }
        TypeKind::EntityType(ins, outs) => {
            let ins = ins.iter().map(type_expr).collect::<Vec<_>>();
            let outs = outs.iter().map(type_expr).collect::<Vec<_>>();
            format!("(EntityTy {} {})", vec_expr(&ins), vec_expr(&outs))
        }
    }
}

fn vec_expr(elems: &[String]) -> String {
    if elems.is_empty() {
        "(vec-empty)".to_string()
    } else {
        format!("(vec-of {})", elems.join(" "))
    }
}

fn opcode_symbol(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::ConstInt => "ConstInt",
        Opcode::ConstTime => "ConstTime",
        Opcode::Alias => "Alias",
        Opcode::ArrayUniform => "ArrayUniform",
        Opcode::Array => "Array",
        Opcode::Struct => "Struct",
        Opcode::Not => "Not",
        Opcode::Neg => "Neg",
        Opcode::Add => "Add",
        Opcode::Sub => "Sub",
        Opcode::And => "And",
        Opcode::Or => "Or",
        Opcode::Xor => "Xor",
        Opcode::Smul => "Smul",
        Opcode::Sdiv => "Sdiv",
        Opcode::Smod => "Smod",
        Opcode::Srem => "Srem",
        Opcode::Umul => "Umul",
        Opcode::Udiv => "Udiv",
        Opcode::Umod => "Umod",
        Opcode::Urem => "Urem",
        Opcode::Eq => "Eq",
        Opcode::Neq => "Neq",
        Opcode::Slt => "Slt",
        Opcode::Sgt => "Sgt",
        Opcode::Sle => "Sle",
        Opcode::Sge => "Sge",
        Opcode::Ult => "Ult",
        Opcode::Ugt => "Ugt",
        Opcode::Ule => "Ule",
        Opcode::Uge => "Uge",
        Opcode::Shl => "Shl",
        Opcode::Shr => "Shr",
        Opcode::Mux => "Mux",
        Opcode::InsField => "InsField",
        Opcode::InsSlice => "InsSlice",
        Opcode::ExtField => "ExtField",
        Opcode::ExtSlice => "ExtSlice",
        _ => "ValueRef",
    }
}

fn format_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn escape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
