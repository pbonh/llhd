use crate::ir::{
    Block, CfgSkeleton, EClassRef, ExtUnit, InstData, Opcode, RegMode, Signature, Unit,
    UnitBuilder, UnitData, UnitEGraph, UnitKind, UnitName, Value, CFG_SK_BLOCK, CFG_SK_BLOCK_ARG,
    CFG_SK_EFFECT, CFG_SK_TERM_BR, CFG_SK_TERM_BR_COND, CFG_SK_TERM_HALT, CFG_SK_TERM_RET,
    CFG_SK_TERM_RET_VALUE, CFG_SK_TERM_WAIT, CFG_SK_TERM_WAIT_TIME,
};
use crate::table::TableKey;
use crate::ty::{
    array_ty, enum_ty, func_ty, int_ty, pointer_ty, signal_ty, struct_ty, time_ty, void_ty, Type,
    TypeKind,
};
use crate::value::{IntValue, TimeValue};
use anyhow::{anyhow, bail, Context, Result};
use egglog_numeric_id::NumericId;
use num::{BigInt, BigRational, BigUint, Zero};
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum Term {
    Atom(String),
    Str(String),
    List(Vec<Term>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgDir {
    Input,
    Output,
}

#[derive(Debug, Clone)]
struct ArgValueEntry {
    dir: ArgDir,
    index: usize,
    value_id: usize,
}

#[derive(Debug, Clone)]
struct ParsedInst {
    inst_id: usize,
    block_id: usize,
    opcode: Opcode,
    result: Option<usize>,
    ty: Type,
    args: Vec<ParsedValue>,
    blocks: Vec<usize>,
    imms: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedValue {
    Invalid,
    Id(usize),
}

#[derive(Debug, Clone)]
struct ConstIntInfo {
    width: usize,
    value: BigUint,
}

#[derive(Debug, Clone)]
struct ConstTimeInfo {
    num: BigInt,
    den: BigInt,
    delta: usize,
    epsilon: usize,
}

#[derive(Debug, Clone)]
struct CallInfo {
    ext_id: usize,
    ins: u16,
}

#[derive(Debug, Clone)]
struct ParsedExtUnit {
    name: UnitName,
    sig: Signature,
}

#[derive(Debug, Clone)]
struct ParsedUnit {
    kind: UnitKind,
    name: UnitName,
    sig: Signature,
}

#[derive(Debug, Default)]
struct ParsedProgram {
    unit: Option<ParsedUnit>,
    arg_values: Vec<ArgValueEntry>,
    ext_units: HashMap<usize, ParsedExtUnit>,
    block_order: Vec<usize>,
    insts: Vec<ParsedInst>,
    const_ints: HashMap<usize, ConstIntInfo>,
    const_times: HashMap<usize, ConstTimeInfo>,
    call_info: HashMap<usize, CallInfo>,
    reg_modes: HashMap<usize, Vec<RegMode>>,
}

/// Serialize a unit into an egglog-style program.
pub fn unit_to_egglog_program(unit: &Unit<'_>) -> Result<String> {
    let mut lines = Vec::new();
    let (name_tag, name_val) = unit_name_terms(unit.name());
    let inputs = types_list(
        unit.sig()
            .inputs()
            .map(|arg| unit.sig().arg_type(arg))
            .collect(),
    );
    let outputs = types_list(
        unit.sig()
            .outputs()
            .map(|arg| unit.sig().arg_type(arg))
            .collect(),
    );
    let ret = unit
        .sig()
        .has_return_type()
        .then(|| type_term(&unit.sig().return_type()))
        .unwrap_or_else(|| Term::Atom("None".into()));
    let kind = Term::Atom(unit_kind_atom(unit.kind()).to_string());
    lines.push(format_term(&Term::List(vec![
        Term::Atom("Unit".into()),
        kind,
        name_tag,
        name_val,
        inputs.clone(),
        outputs.clone(),
        ret.clone(),
    ])));

    for (index, value) in unit.input_args().enumerate() {
        lines.push(format_term(&Term::List(vec![
            Term::Atom("ArgValue".into()),
            Term::Atom("input".into()),
            Term::Atom(index.to_string()),
            Term::Atom(value.index().to_string()),
        ])));
    }
    for (index, value) in unit.output_args().enumerate() {
        lines.push(format_term(&Term::List(vec![
            Term::Atom("ArgValue".into()),
            Term::Atom("output".into()),
            Term::Atom(index.to_string()),
            Term::Atom(value.index().to_string()),
        ])));
    }

    for (ext, data) in unit.extern_units() {
        let (ext_tag, ext_name) = unit_name_terms(&data.name);
        let ext_inputs = types_list(
            data.sig
                .inputs()
                .map(|arg| data.sig.arg_type(arg))
                .collect(),
        );
        let ext_outputs = types_list(
            data.sig
                .outputs()
                .map(|arg| data.sig.arg_type(arg))
                .collect(),
        );
        let ext_ret = data
            .sig
            .has_return_type()
            .then(|| type_term(&data.sig.return_type()))
            .unwrap_or_else(|| Term::Atom("None".into()));
        lines.push(format_term(&Term::List(vec![
            Term::Atom("ExtUnit".into()),
            Term::Atom(ext.index().to_string()),
            ext_tag,
            ext_name,
            ext_inputs,
            ext_outputs,
            ext_ret,
        ])));
    }

    let block_ids: Vec<_> = unit.blocks().map(|bb| bb.index()).collect();
    let mut block_order = vec![Term::Atom("BlockOrder".into())];
    block_order.extend(block_ids.iter().map(|id| Term::Atom(id.to_string())));
    lines.push(format_term(&Term::List(block_order)));

    for bb in unit.blocks() {
        for inst in unit.insts(bb) {
            let data = &unit[inst];
            let opcode = Term::Atom(opcode_atom(data.opcode()).to_string());
            let result = unit
                .get_inst_result(inst)
                .map(|value| Term::Atom(value.index().to_string()))
                .unwrap_or_else(|| Term::Atom("none".into()));
            let ty = type_term(&unit.inst_type(inst));
            let args = values_list(
                data.args()
                    .iter()
                    .map(|arg| parsed_value_term(*arg))
                    .collect(),
            );
            let blocks = values_list(
                data.blocks()
                    .iter()
                    .map(|bb| Term::Atom(bb.index().to_string()))
                    .collect(),
            );
            let imms = values_list(
                data.imms()
                    .iter()
                    .map(|imm| Term::Atom(imm.to_string()))
                    .collect(),
            );
            lines.push(format_term(&Term::List(vec![
                Term::Atom("Inst".into()),
                Term::Atom(inst.index().to_string()),
                Term::Atom(bb.index().to_string()),
                opcode,
                result,
                ty,
                args,
                blocks,
                imms,
            ])));

            if let Some(imm) = data.get_const_int() {
                lines.push(format_term(&Term::List(vec![
                    Term::Atom("ConstInt".into()),
                    Term::Atom(inst.index().to_string()),
                    Term::Atom(imm.width.to_string()),
                    Term::Str(imm.value.to_string()),
                ])));
            }
            if let Some(imm) = data.get_const_time() {
                let num = imm.time.numer().clone();
                let den = imm.time.denom().clone();
                lines.push(format_term(&Term::List(vec![
                    Term::Atom("ConstTime".into()),
                    Term::Atom(inst.index().to_string()),
                    Term::Str(num.to_string()),
                    Term::Str(den.to_string()),
                    Term::Atom(imm.delta.to_string()),
                    Term::Atom(imm.epsilon.to_string()),
                ])));
            }
            if let Some(ext) = data.get_ext_unit() {
                if matches!(data.opcode(), Opcode::Call | Opcode::Inst) {
                    let ins = data.input_args().len() as u16;
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom("CallInfo".into()),
                        Term::Atom(inst.index().to_string()),
                        Term::Atom(ext.index().to_string()),
                        Term::Atom(ins.to_string()),
                    ])));
                }
            }
            if data.opcode() == Opcode::Reg {
                let modes: Vec<_> = data
                    .mode_args()
                    .map(|mode| Term::Atom(reg_mode_atom(mode)))
                    .collect();
                lines.push(format_term(&Term::List(vec![
                    Term::Atom("RegModes".into()),
                    Term::Atom(inst.index().to_string()),
                    Term::List(modes),
                ])));
            }
        }
    }

    let mut egraph = UnitEGraph::build_from_unit(unit)?;
    let skeleton = CfgSkeleton::build_from_unit(unit, &mut egraph)?;
    for block in &skeleton.blocks {
        lines.push(format_term(&Term::List(vec![
            Term::Atom(CFG_SK_BLOCK.into()),
            Term::Atom(block.block.index().to_string()),
        ])));
        for arg in &block.args {
            lines.push(format_term(&Term::List(vec![
                Term::Atom(CFG_SK_BLOCK_ARG.into()),
                Term::Atom(block.block.index().to_string()),
                Term::Atom(arg.value.index().to_string()),
                Term::Atom(eclass_id(arg.class).to_string()),
            ])));
        }
        for stmt in &block.stmts {
            match stmt {
                crate::ir::SkeletonStmt::Effect {
                    inst,
                    opcode,
                    args,
                    result,
                } => {
                    let arg_list = values_list(
                        args.iter()
                            .map(|arg| Term::Atom(eclass_id(*arg).to_string()))
                            .collect(),
                    );
                    let result = result
                        .map(|value| Term::Atom(eclass_id(value).to_string()))
                        .unwrap_or_else(|| Term::Atom("none".into()));
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_EFFECT.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                        Term::Atom(opcode_atom(*opcode).to_string()),
                        result,
                        arg_list,
                    ])));
                }
            }
        }
        if let Some(term) = &block.terminator {
            match term {
                crate::ir::SkeletonTerminator::Br { inst, target, args } => {
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_TERM_BR.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                        Term::Atom(target.index().to_string()),
                        values_list(
                            args.iter()
                                .map(|arg| Term::Atom(eclass_id(*arg).to_string()))
                                .collect(),
                        ),
                    ])));
                }
                crate::ir::SkeletonTerminator::BrCond {
                    inst,
                    cond,
                    then_target,
                    then_args,
                    else_target,
                    else_args,
                } => {
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_TERM_BR_COND.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                        Term::Atom(eclass_id(*cond).to_string()),
                        Term::Atom(then_target.index().to_string()),
                        values_list(
                            then_args
                                .iter()
                                .map(|arg| Term::Atom(eclass_id(*arg).to_string()))
                                .collect(),
                        ),
                        Term::Atom(else_target.index().to_string()),
                        values_list(
                            else_args
                                .iter()
                                .map(|arg| Term::Atom(eclass_id(*arg).to_string()))
                                .collect(),
                        ),
                    ])));
                }
                crate::ir::SkeletonTerminator::Wait { inst, target, args } => {
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_TERM_WAIT.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                        Term::Atom(target.index().to_string()),
                        values_list(
                            args.iter()
                                .map(|arg| Term::Atom(eclass_id(*arg).to_string()))
                                .collect(),
                        ),
                    ])));
                }
                crate::ir::SkeletonTerminator::WaitTime {
                    inst,
                    time,
                    target,
                    args,
                } => {
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_TERM_WAIT_TIME.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                        Term::Atom(eclass_id(*time).to_string()),
                        Term::Atom(target.index().to_string()),
                        values_list(
                            args.iter()
                                .map(|arg| Term::Atom(eclass_id(*arg).to_string()))
                                .collect(),
                        ),
                    ])));
                }
                crate::ir::SkeletonTerminator::Ret { inst } => {
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_TERM_RET.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                    ])));
                }
                crate::ir::SkeletonTerminator::RetValue { inst, value } => {
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_TERM_RET_VALUE.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                        Term::Atom(eclass_id(*value).to_string()),
                    ])));
                }
                crate::ir::SkeletonTerminator::Halt { inst } => {
                    lines.push(format_term(&Term::List(vec![
                        Term::Atom(CFG_SK_TERM_HALT.into()),
                        Term::Atom(block.block.index().to_string()),
                        Term::Atom(inst.index().to_string()),
                    ])));
                }
            }
        }
    }

    Ok(lines.join("\n"))
}

/// Dump a human-readable egglog program with schema notes.
#[cfg(feature = "egglog-debug")]
pub fn dump_egglog_debug(unit: &Unit<'_>, program: &str) -> String {
    let mut out = String::new();
    out.push_str("LLHD Egglog Debug Dump\n");
    out.push_str("======================\n\n");
    out.push_str(&format!("unit: {}\n", unit.name()));
    out.push_str(&format!("kind: {:?}\n", unit.kind()));
    out.push_str(&format!("signature: {}\n\n", unit.sig()));

    out.push_str("Schema (serialization terms)\n");
    out.push_str("-----------------------------\n");
    out.push_str("Unit(kind, name_tag, name_value, inputs, outputs, return)\n");
    out.push_str("ArgValue(dir, index, value_id)\n");
    out.push_str("ExtUnit(id, name_tag, name_value, inputs, outputs, return)\n");
    out.push_str("BlockOrder(block_id...)\n");
    out.push_str("Inst(inst_id, block_id, opcode, result, type, args, blocks, imms)\n");
    out.push_str("ConstInt(inst_id, width, value)\n");
    out.push_str("ConstTime(inst_id, num, den, delta, epsilon)\n");
    out.push_str("CallInfo(inst_id, ext_id, ins)\n");
    out.push_str("RegModes(inst_id, [modes...])\n\n");

    out.push_str("CFG skeleton schema\n");
    out.push_str("-------------------\n");
    let cfg_schema = crate::ir::cfg_schema_program();
    out.push_str(cfg_schema);
    if !cfg_schema.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');

    out.push_str("Facts (egglog program)\n");
    out.push_str("----------------------\n");
    out.push_str(program);
    out.push('\n');
    out
}

/// Deserialize a unit from an egglog-style program.
pub fn unit_from_egglog_program(program: &str) -> Result<UnitData> {
    let parsed = parse_program(program)?;
    let unit = parsed.unit.ok_or_else(|| anyhow!("missing Unit entry"))?;

    let mut data = UnitData {
        kind: unit.kind,
        name: unit.name,
        sig: unit.sig,
        dfg: Default::default(),
        cfg: Default::default(),
        layout: Default::default(),
        cfg_skeleton: Default::default(),
        egraph: Default::default(),
    };

    let mut builder = UnitBuilder::new_anonymous(&mut data);
    let sig = builder.sig().clone();
    builder.make_args_for_signature(&sig);

    let mut input_values: Vec<Option<usize>> = vec![None; sig.inputs().count()];
    let mut output_values: Vec<Option<usize>> = vec![None; sig.outputs().count()];
    for entry in &parsed.arg_values {
        match entry.dir {
            ArgDir::Input => {
                if entry.index >= input_values.len() {
                    bail!("input arg index out of range");
                }
                input_values[entry.index] = Some(entry.value_id);
            }
            ArgDir::Output => {
                if entry.index >= output_values.len() {
                    bail!("output arg index out of range");
                }
                output_values[entry.index] = Some(entry.value_id);
            }
        }
    }

    let mut value_types: HashMap<usize, Type> = HashMap::new();
    for (idx, arg) in sig.inputs().enumerate() {
        let value_id = input_values
            .get(idx)
            .and_then(|value| *value)
            .ok_or_else(|| anyhow!("missing input arg value mapping"))?;
        value_types.insert(value_id, sig.arg_type(arg));
    }
    for (idx, arg) in sig.outputs().enumerate() {
        let value_id = output_values
            .get(idx)
            .and_then(|value| *value)
            .ok_or_else(|| anyhow!("missing output arg value mapping"))?;
        value_types.insert(value_id, sig.arg_type(arg));
    }
    for inst in &parsed.insts {
        if let Some(result) = inst.result {
            if let Some(existing) = value_types.get(&result) {
                if existing != &inst.ty {
                    bail!("value type mismatch for serialized value {}", result);
                }
            } else {
                value_types.insert(result, inst.ty.clone());
            }
        }
    }

    let mut value_map: HashMap<usize, (Value, bool)> = HashMap::new();
    for (idx, arg) in sig.inputs().enumerate() {
        let value_id = input_values[idx].unwrap();
        value_map.insert(value_id, (builder.input_arg(idx), false));
        value_types.insert(value_id, sig.arg_type(arg));
    }
    for (idx, arg) in sig.outputs().enumerate() {
        let value_id = output_values[idx].unwrap();
        value_map.insert(value_id, (builder.output_arg(idx), false));
        value_types.insert(value_id, sig.arg_type(arg));
    }

    let mut ext_map = HashMap::new();
    for (id, ext) in parsed.ext_units {
        let ext_unit = builder.add_extern(ext.name, ext.sig);
        ext_map.insert(id, ext_unit);
    }

    let mut block_map = HashMap::new();
    for block_id in parsed.block_order {
        let bb = builder.block();
        block_map.insert(block_id, bb);
    }

    for inst in parsed.insts {
        let bb = *block_map
            .get(&inst.block_id)
            .ok_or_else(|| anyhow!("missing block {}", inst.block_id))?;
        builder.append_to(bb);

        let args = resolve_values(&mut builder, &value_types, &mut value_map, &inst.args)?;
        let blocks = resolve_blocks(&block_map, &inst.blocks)?;
        let imms = inst.imms;
        let inst_data = build_inst_data(
            inst.opcode,
            args,
            blocks,
            imms,
            &parsed.const_ints,
            &parsed.const_times,
            &parsed.call_info,
            &parsed.reg_modes,
            &ext_map,
            inst.inst_id,
        )?;
        let created = builder.build_inst(inst_data, inst.ty.clone());
        if let Some(result_id) = inst.result {
            let result = builder.inst_result(created);
            match value_map.get(&result_id) {
                None => {
                    value_map.insert(result_id, (result, false));
                }
                Some((existing, is_placeholder)) => {
                    if !*is_placeholder {
                        bail!("duplicate value definition for {}", result_id);
                    }
                    builder.replace_use(*existing, result);
                    builder.remove_placeholder(*existing);
                    value_map.insert(result_id, (result, false));
                }
            }
        }
    }

    let unresolved: Vec<_> = value_map
        .iter()
        .filter_map(|(id, (_, placeholder))| (*placeholder).then_some(*id))
        .collect();
    if !unresolved.is_empty() {
        bail!("unresolved values: {:?}", unresolved);
    }

    builder.finish_rebuild()?;
    Ok(data)
}

fn build_inst_data(
    opcode: Opcode,
    args: Vec<Value>,
    blocks: Vec<Block>,
    imms: Vec<usize>,
    const_ints: &HashMap<usize, ConstIntInfo>,
    const_times: &HashMap<usize, ConstTimeInfo>,
    call_info: &HashMap<usize, CallInfo>,
    reg_modes: &HashMap<usize, Vec<RegMode>>,
    ext_map: &HashMap<usize, ExtUnit>,
    inst_id: usize,
) -> Result<InstData> {
    Ok(match opcode {
        Opcode::ConstInt => {
            let info = const_ints
                .get(&inst_id)
                .ok_or_else(|| anyhow!("missing ConstInt for inst {}", inst_id))?;
            InstData::ConstInt {
                opcode,
                imm: IntValue::from_unsigned(info.width, info.value.clone()),
            }
        }
        Opcode::ConstTime => {
            let info = const_times
                .get(&inst_id)
                .ok_or_else(|| anyhow!("missing ConstTime for inst {}", inst_id))?;
            let den = if info.den.is_zero() {
                BigInt::from(1)
            } else {
                info.den.clone()
            };
            let time = BigRational::new(info.num.clone(), den);
            InstData::ConstTime {
                opcode,
                imm: TimeValue::new(time, info.delta, info.epsilon),
            }
        }
        Opcode::ArrayUniform => InstData::Array {
            opcode,
            imms: [get_imm(&imms, 0, inst_id)?],
            args: [get_arg(&args, 0, inst_id)?],
        },
        Opcode::Array | Opcode::Struct => InstData::Aggregate { opcode, args },
        Opcode::Alias
        | Opcode::Not
        | Opcode::Neg
        | Opcode::Sig
        | Opcode::Prb
        | Opcode::Var
        | Opcode::Ld
        | Opcode::RetValue => InstData::Unary {
            opcode,
            args: [get_arg(&args, 0, inst_id)?],
        },
        Opcode::Add
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
        | Opcode::Mux
        | Opcode::Con
        | Opcode::St => InstData::Binary {
            opcode,
            args: [get_arg(&args, 0, inst_id)?, get_arg(&args, 1, inst_id)?],
        },
        Opcode::Shl | Opcode::Shr | Opcode::Del | Opcode::Drv => InstData::Ternary {
            opcode,
            args: [
                get_arg(&args, 0, inst_id)?,
                get_arg(&args, 1, inst_id)?,
                get_arg(&args, 2, inst_id)?,
            ],
        },
        Opcode::DrvCond => InstData::Quaternary {
            opcode,
            args: [
                get_arg(&args, 0, inst_id)?,
                get_arg(&args, 1, inst_id)?,
                get_arg(&args, 2, inst_id)?,
                get_arg(&args, 3, inst_id)?,
            ],
        },
        Opcode::InsField | Opcode::InsSlice | Opcode::ExtField | Opcode::ExtSlice => {
            let arg0 = get_arg(&args, 0, inst_id)?;
            let arg1 = args.get(1).copied().unwrap_or_else(Value::invalid);
            let imm0 = get_imm(&imms, 0, inst_id)?;
            let imm1 = match opcode {
                Opcode::InsField | Opcode::ExtField => 0,
                Opcode::InsSlice | Opcode::ExtSlice => get_imm(&imms, 1, inst_id)?,
                _ => 0,
            };
            InstData::InsExt {
                opcode,
                args: [arg0, arg1],
                imms: [imm0, imm1],
            }
        }
        Opcode::Call | Opcode::Inst => {
            let info = call_info
                .get(&inst_id)
                .ok_or_else(|| anyhow!("missing CallInfo for inst {}", inst_id))?;
            let ext = *ext_map
                .get(&info.ext_id)
                .ok_or_else(|| anyhow!("missing ext unit {}", info.ext_id))?;
            InstData::Call {
                opcode,
                unit: ext,
                ins: info.ins,
                args,
            }
        }
        Opcode::Reg => {
            let modes = reg_modes
                .get(&inst_id)
                .cloned()
                .ok_or_else(|| anyhow!("missing RegModes for inst {}", inst_id))?;
            InstData::Reg {
                opcode,
                args,
                modes,
            }
        }
        Opcode::Phi => InstData::Phi {
            opcode,
            args,
            bbs: blocks,
        },
        Opcode::Br => InstData::Jump {
            opcode,
            bbs: [get_block(&blocks, 0, inst_id)?],
        },
        Opcode::BrCond => InstData::Branch {
            opcode,
            args: [get_arg(&args, 0, inst_id)?],
            bbs: [
                get_block(&blocks, 0, inst_id)?,
                get_block(&blocks, 1, inst_id)?,
            ],
        },
        Opcode::Wait | Opcode::WaitTime => InstData::Wait {
            opcode,
            bbs: [get_block(&blocks, 0, inst_id)?],
            args,
        },
        Opcode::Halt | Opcode::Ret => InstData::Nullary { opcode },
    })
}

fn get_arg(args: &[Value], idx: usize, inst_id: usize) -> Result<Value> {
    args.get(idx)
        .copied()
        .ok_or_else(|| anyhow!("missing arg {} for inst {}", idx, inst_id))
}

fn get_block(blocks: &[Block], idx: usize, inst_id: usize) -> Result<Block> {
    blocks
        .get(idx)
        .copied()
        .ok_or_else(|| anyhow!("missing block {} for inst {}", idx, inst_id))
}

fn get_imm(imms: &[usize], idx: usize, inst_id: usize) -> Result<usize> {
    imms.get(idx)
        .copied()
        .ok_or_else(|| anyhow!("missing imm {} for inst {}", idx, inst_id))
}

fn resolve_values(
    builder: &mut UnitBuilder<'_>,
    value_types: &HashMap<usize, Type>,
    value_map: &mut HashMap<usize, (Value, bool)>,
    args: &[ParsedValue],
) -> Result<Vec<Value>> {
    let mut out = Vec::with_capacity(args.len());
    for arg in args {
        match arg {
            ParsedValue::Invalid => out.push(Value::invalid()),
            ParsedValue::Id(value_id) => {
                if let Some((value, _)) = value_map.get(value_id) {
                    out.push(*value);
                } else {
                    let ty = value_types
                        .get(value_id)
                        .ok_or_else(|| anyhow!("missing type for value {}", value_id))?;
                    let value = builder.add_placeholder(ty.clone());
                    value_map.insert(*value_id, (value, true));
                    out.push(value);
                }
            }
        }
    }
    Ok(out)
}

fn resolve_blocks(block_map: &HashMap<usize, Block>, blocks: &[usize]) -> Result<Vec<Block>> {
    blocks
        .iter()
        .map(|id| {
            block_map
                .get(id)
                .copied()
                .ok_or_else(|| anyhow!("missing block {}", id))
        })
        .collect()
}

fn parse_program(program: &str) -> Result<ParsedProgram> {
    let mut parsed = ParsedProgram::default();
    for (line_no, line) in program.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let term =
            parse_line(line).with_context(|| format!("parse error on line {}", line_no + 1))?;
        let Term::List(items) = term else {
            bail!("expected list on line {}", line_no + 1);
        };
        let (head, rest) = items.split_first().ok_or_else(|| anyhow!("empty term"))?;
        let head = term_atom(head)?;
        match head.as_str() {
            "Unit" => {
                let (kind, name, sig) = parse_unit_entry(rest)?;
                parsed.unit = Some(ParsedUnit { kind, name, sig });
            }
            "ArgValue" => parsed.arg_values.push(parse_arg_value(rest)?),
            "ExtUnit" => {
                let (id, ext) = parse_ext_unit(rest)?;
                parsed.ext_units.insert(id, ext);
            }
            "BlockOrder" => {
                parsed.block_order = rest
                    .iter()
                    .map(|term| parse_usize(term))
                    .collect::<Result<_>>()?;
            }
            "Inst" => parsed.insts.push(parse_inst(rest)?),
            "ConstInt" => {
                let (inst_id, info) = parse_const_int(rest)?;
                parsed.const_ints.insert(inst_id, info);
            }
            "ConstTime" => {
                let (inst_id, info) = parse_const_time(rest)?;
                parsed.const_times.insert(inst_id, info);
            }
            "CallInfo" => {
                let info = parse_call_info(rest)?;
                parsed.call_info.insert(info.0, info.1);
            }
            "RegModes" => {
                let (inst_id, modes) = parse_reg_modes(rest)?;
                parsed.reg_modes.insert(inst_id, modes);
            }
            CFG_SK_BLOCK
            | CFG_SK_BLOCK_ARG
            | CFG_SK_EFFECT
            | CFG_SK_TERM_BR
            | CFG_SK_TERM_BR_COND
            | CFG_SK_TERM_WAIT
            | CFG_SK_TERM_WAIT_TIME
            | CFG_SK_TERM_RET
            | CFG_SK_TERM_RET_VALUE
            | CFG_SK_TERM_HALT => {
                // Skeleton entries are parsed but not required for reconstruction.
            }
            other => bail!("unknown term {}", other),
        }
    }
    Ok(parsed)
}

fn parse_unit_entry(items: &[Term]) -> Result<(UnitKind, UnitName, Signature)> {
    if items.len() != 6 {
        bail!("Unit entry expects 6 fields");
    }
    let kind = parse_unit_kind(&term_atom(&items[0])?)?;
    let name_tag = term_atom(&items[1])?;
    let name = parse_unit_name(&name_tag, &items[2])?;
    let inputs = parse_type_list(&items[3])?;
    let outputs = parse_type_list(&items[4])?;
    let ret = parse_optional_type(&items[5])?;
    Ok((kind, name, signature_from_parts(inputs, outputs, ret)))
}

fn parse_arg_value(items: &[Term]) -> Result<ArgValueEntry> {
    if items.len() != 3 {
        bail!("ArgValue entry expects 3 fields");
    }
    let dir = match term_atom(&items[0])?.as_str() {
        "input" => ArgDir::Input,
        "output" => ArgDir::Output,
        other => bail!("unknown ArgValue dir {}", other),
    };
    Ok(ArgValueEntry {
        dir,
        index: parse_usize(&items[1])?,
        value_id: parse_usize(&items[2])?,
    })
}

fn parse_ext_unit(items: &[Term]) -> Result<(usize, ParsedExtUnit)> {
    if items.len() != 6 {
        bail!("ExtUnit entry expects 6 fields");
    }
    let id = parse_usize(&items[0])?;
    let name_tag = term_atom(&items[1])?;
    let name = parse_unit_name(&name_tag, &items[2])?;
    let inputs = parse_type_list(&items[3])?;
    let outputs = parse_type_list(&items[4])?;
    let ret = parse_optional_type(&items[5])?;
    let sig = signature_from_parts(inputs, outputs, ret);
    Ok((id, ParsedExtUnit { name, sig }))
}

fn parse_inst(items: &[Term]) -> Result<ParsedInst> {
    if items.len() != 8 {
        bail!("Inst entry expects 8 fields");
    }
    let inst_id = parse_usize(&items[0])?;
    let block_id = parse_usize(&items[1])?;
    let opcode = parse_opcode(&term_atom(&items[2])?)?;
    let result = match term_atom(&items[3])?.as_str() {
        "none" => None,
        other => Some(other.parse::<usize>().context("invalid result id")?),
    };
    let ty = parse_type(&items[4])?;
    let args = parse_value_list(&items[5])?;
    let blocks = parse_usize_list(&items[6])?;
    let imms = parse_usize_list(&items[7])?;
    Ok(ParsedInst {
        inst_id,
        block_id,
        opcode,
        result,
        ty,
        args,
        blocks,
        imms,
    })
}

fn parse_const_int(items: &[Term]) -> Result<(usize, ConstIntInfo)> {
    if items.len() != 3 {
        bail!("ConstInt entry expects 3 fields");
    }
    let inst_id = parse_usize(&items[0])?;
    let width = parse_usize(&items[1])?;
    let value_str = term_string(&items[2])?;
    let value = BigUint::parse_bytes(value_str.as_bytes(), 10)
        .ok_or_else(|| anyhow!("invalid ConstInt value"))?;
    Ok((inst_id, ConstIntInfo { width, value }))
}

fn parse_const_time(items: &[Term]) -> Result<(usize, ConstTimeInfo)> {
    if items.len() != 5 {
        bail!("ConstTime entry expects 5 fields");
    }
    let inst_id = parse_usize(&items[0])?;
    let num = parse_bigint(term_string(&items[1])?)?;
    let den = parse_bigint(term_string(&items[2])?)?;
    let delta = parse_usize(&items[3])?;
    let epsilon = parse_usize(&items[4])?;
    Ok((
        inst_id,
        ConstTimeInfo {
            num,
            den,
            delta,
            epsilon,
        },
    ))
}

fn parse_call_info(items: &[Term]) -> Result<(usize, CallInfo)> {
    if items.len() != 3 {
        bail!("CallInfo entry expects 3 fields");
    }
    let inst_id = parse_usize(&items[0])?;
    let ext_id = parse_usize(&items[1])?;
    let ins = parse_usize(&items[2])?;
    Ok((
        inst_id,
        CallInfo {
            ext_id,
            ins: ins as u16,
        },
    ))
}

fn parse_reg_modes(items: &[Term]) -> Result<(usize, Vec<RegMode>)> {
    if items.len() != 2 {
        bail!("RegModes entry expects 2 fields");
    }
    let inst_id = parse_usize(&items[0])?;
    let modes = match &items[1] {
        Term::List(values) => values
            .iter()
            .map(|term| parse_reg_mode(&term_atom(term)?))
            .collect::<Result<Vec<_>>>()?,
        _ => bail!("RegModes expects list"),
    };
    Ok((inst_id, modes))
}

fn parse_value_list(term: &Term) -> Result<Vec<ParsedValue>> {
    let Term::List(items) = term else {
        bail!("expected list");
    };
    items
        .iter()
        .map(|item| match term_atom(item)?.as_str() {
            "invalid" => Ok(ParsedValue::Invalid),
            other => Ok(ParsedValue::Id(
                other.parse::<usize>().context("invalid value id")?,
            )),
        })
        .collect()
}

fn parse_usize_list(term: &Term) -> Result<Vec<usize>> {
    let Term::List(items) = term else {
        bail!("expected list");
    };
    items.iter().map(parse_usize).collect()
}

fn parse_optional_type(term: &Term) -> Result<Option<Type>> {
    if let Term::Atom(atom) = term {
        if atom == "None" {
            return Ok(None);
        }
    }
    Ok(Some(parse_type(term)?))
}

fn parse_type_list(term: &Term) -> Result<Vec<Type>> {
    let Term::List(items) = term else {
        bail!("expected type list");
    };
    items.iter().map(parse_type).collect()
}

fn parse_type(term: &Term) -> Result<Type> {
    match term {
        Term::Atom(atom) => match atom.as_str() {
            "Void" => Ok(void_ty()),
            "Time" => Ok(time_ty()),
            other => bail!("unknown type atom {}", other),
        },
        Term::List(items) => {
            let (head, rest) = items
                .split_first()
                .ok_or_else(|| anyhow!("empty type term"))?;
            let head = term_atom(head)?;
            match head.as_str() {
                "IntTy" => Ok(int_ty(parse_usize(single(rest, "IntTy")?)?)),
                "Enum" => Ok(enum_ty(parse_usize(single(rest, "Enum")?)?)),
                "Pointer" => Ok(pointer_ty(parse_type(single_term(rest, "Pointer")?)?)),
                "Signal" => Ok(signal_ty(parse_type(single_term(rest, "Signal")?)?)),
                "ArrayTy" => {
                    if rest.len() != 2 {
                        bail!("ArrayTy expects 2 fields");
                    }
                    let len = parse_usize(&rest[0])?;
                    let ty = parse_type(&rest[1])?;
                    Ok(array_ty(len, ty))
                }
                "StructTy" => {
                    if rest.len() != 1 {
                        bail!("StructTy expects list field");
                    }
                    let fields = parse_type_list(&rest[0])?;
                    Ok(struct_ty(fields))
                }
                "FuncTy" => {
                    if rest.len() != 2 {
                        bail!("FuncTy expects args list and return type");
                    }
                    let args = parse_type_list(&rest[0])?;
                    let ret = parse_type(&rest[1])?;
                    Ok(func_ty(args, ret))
                }
                "EntityTy" => {
                    if rest.len() != 2 {
                        bail!("EntityTy expects inputs and outputs list");
                    }
                    let ins = parse_type_list(&rest[0])?;
                    let outs = parse_type_list(&rest[1])?;
                    Ok(crate::ty::entity_ty(ins, outs))
                }
                other => bail!("unknown type form {}", other),
            }
        }
        Term::Str(_) => bail!("unexpected string in type"),
    }
}

fn single<'a>(rest: &'a [Term], name: &str) -> Result<&'a Term> {
    if rest.len() != 1 {
        bail!("{} expects 1 field", name);
    }
    Ok(&rest[0])
}

fn single_term<'a>(rest: &'a [Term], name: &str) -> Result<&'a Term> {
    single(rest, name)
}

fn parse_unit_kind(atom: &str) -> Result<UnitKind> {
    match atom {
        "function" => Ok(UnitKind::Function),
        "process" => Ok(UnitKind::Process),
        "entity" => Ok(UnitKind::Entity),
        other => bail!("unknown unit kind {}", other),
    }
}

fn unit_kind_atom(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Function => "function",
        UnitKind::Process => "process",
        UnitKind::Entity => "entity",
    }
}

fn opcode_atom(opcode: Opcode) -> &'static str {
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
        Opcode::Reg => "Reg",
        Opcode::InsField => "InsField",
        Opcode::InsSlice => "InsSlice",
        Opcode::ExtField => "ExtField",
        Opcode::ExtSlice => "ExtSlice",
        Opcode::Con => "Con",
        Opcode::Del => "Del",
        Opcode::Call => "Call",
        Opcode::Inst => "Inst",
        Opcode::Sig => "Sig",
        Opcode::Prb => "Prb",
        Opcode::Drv => "Drv",
        Opcode::DrvCond => "DrvCond",
        Opcode::Var => "Var",
        Opcode::Ld => "Ld",
        Opcode::St => "St",
        Opcode::Halt => "Halt",
        Opcode::Ret => "Ret",
        Opcode::RetValue => "RetValue",
        Opcode::Phi => "Phi",
        Opcode::Br => "Br",
        Opcode::BrCond => "BrCond",
        Opcode::Wait => "Wait",
        Opcode::WaitTime => "WaitTime",
    }
}

fn parse_opcode(atom: &str) -> Result<Opcode> {
    Ok(match atom {
        "ConstInt" => Opcode::ConstInt,
        "ConstTime" => Opcode::ConstTime,
        "Alias" => Opcode::Alias,
        "ArrayUniform" => Opcode::ArrayUniform,
        "Array" => Opcode::Array,
        "Struct" => Opcode::Struct,
        "Not" => Opcode::Not,
        "Neg" => Opcode::Neg,
        "Add" => Opcode::Add,
        "Sub" => Opcode::Sub,
        "And" => Opcode::And,
        "Or" => Opcode::Or,
        "Xor" => Opcode::Xor,
        "Smul" => Opcode::Smul,
        "Sdiv" => Opcode::Sdiv,
        "Smod" => Opcode::Smod,
        "Srem" => Opcode::Srem,
        "Umul" => Opcode::Umul,
        "Udiv" => Opcode::Udiv,
        "Umod" => Opcode::Umod,
        "Urem" => Opcode::Urem,
        "Eq" => Opcode::Eq,
        "Neq" => Opcode::Neq,
        "Slt" => Opcode::Slt,
        "Sgt" => Opcode::Sgt,
        "Sle" => Opcode::Sle,
        "Sge" => Opcode::Sge,
        "Ult" => Opcode::Ult,
        "Ugt" => Opcode::Ugt,
        "Ule" => Opcode::Ule,
        "Uge" => Opcode::Uge,
        "Shl" => Opcode::Shl,
        "Shr" => Opcode::Shr,
        "Mux" => Opcode::Mux,
        "Reg" => Opcode::Reg,
        "InsField" => Opcode::InsField,
        "InsSlice" => Opcode::InsSlice,
        "ExtField" => Opcode::ExtField,
        "ExtSlice" => Opcode::ExtSlice,
        "Con" => Opcode::Con,
        "Del" => Opcode::Del,
        "Call" => Opcode::Call,
        "Inst" => Opcode::Inst,
        "Sig" => Opcode::Sig,
        "Prb" => Opcode::Prb,
        "Drv" => Opcode::Drv,
        "DrvCond" => Opcode::DrvCond,
        "Var" => Opcode::Var,
        "Ld" => Opcode::Ld,
        "St" => Opcode::St,
        "Halt" => Opcode::Halt,
        "Ret" => Opcode::Ret,
        "RetValue" => Opcode::RetValue,
        "Phi" => Opcode::Phi,
        "Br" => Opcode::Br,
        "BrCond" => Opcode::BrCond,
        "Wait" => Opcode::Wait,
        "WaitTime" => Opcode::WaitTime,
        other => bail!("unknown opcode {}", other),
    })
}

fn reg_mode_atom(mode: RegMode) -> String {
    match mode {
        RegMode::Low => "Low".into(),
        RegMode::High => "High".into(),
        RegMode::Rise => "Rise".into(),
        RegMode::Fall => "Fall".into(),
        RegMode::Both => "Both".into(),
    }
}

fn parse_reg_mode(atom: &str) -> Result<RegMode> {
    Ok(match atom {
        "Low" => RegMode::Low,
        "High" => RegMode::High,
        "Rise" => RegMode::Rise,
        "Fall" => RegMode::Fall,
        "Both" => RegMode::Both,
        other => bail!("unknown reg mode {}", other),
    })
}

fn unit_name_terms(name: &UnitName) -> (Term, Term) {
    match name {
        UnitName::Anonymous(id) => (Term::Atom("anon".into()), Term::Atom(id.to_string())),
        UnitName::Local(name) => (Term::Atom("local".into()), Term::Str(name.clone())),
        UnitName::Global(name) => (Term::Atom("global".into()), Term::Str(name.clone())),
    }
}

fn parse_unit_name(tag: &str, value: &Term) -> Result<UnitName> {
    match tag {
        "anon" => Ok(UnitName::Anonymous(parse_usize(value)? as u32)),
        "local" => Ok(UnitName::Local(term_string(value)?)),
        "global" => Ok(UnitName::Global(term_string(value)?)),
        other => bail!("unknown unit name tag {}", other),
    }
}

fn type_term(ty: &Type) -> Term {
    match ty.as_ref() {
        TypeKind::VoidType => Term::Atom("Void".into()),
        TypeKind::TimeType => Term::Atom("Time".into()),
        TypeKind::IntType(bits) => Term::List(vec![
            Term::Atom("IntTy".into()),
            Term::Atom(bits.to_string()),
        ]),
        TypeKind::EnumType(states) => Term::List(vec![
            Term::Atom("Enum".into()),
            Term::Atom(states.to_string()),
        ]),
        TypeKind::PointerType(inner) => {
            Term::List(vec![Term::Atom("Pointer".into()), type_term(inner)])
        }
        TypeKind::SignalType(inner) => {
            Term::List(vec![Term::Atom("Signal".into()), type_term(inner)])
        }
        TypeKind::ArrayType(len, inner) => Term::List(vec![
            Term::Atom("ArrayTy".into()),
            Term::Atom(len.to_string()),
            type_term(inner),
        ]),
        TypeKind::StructType(fields) => Term::List(vec![
            Term::Atom("StructTy".into()),
            types_list(fields.iter().cloned().collect()),
        ]),
        TypeKind::FuncType(args, ret) => Term::List(vec![
            Term::Atom("FuncTy".into()),
            types_list(args.clone()),
            type_term(ret),
        ]),
        TypeKind::EntityType(ins, outs) => Term::List(vec![
            Term::Atom("EntityTy".into()),
            types_list(ins.clone()),
            types_list(outs.clone()),
        ]),
    }
}

fn types_list(types: Vec<Type>) -> Term {
    Term::List(types.iter().map(type_term).collect())
}

fn values_list(values: Vec<Term>) -> Term {
    Term::List(values)
}

fn parsed_value_term(value: Value) -> Term {
    if value.is_invalid() {
        Term::Atom("invalid".into())
    } else {
        Term::Atom(value.index().to_string())
    }
}

fn signature_from_parts(inputs: Vec<Type>, outputs: Vec<Type>, ret: Option<Type>) -> Signature {
    let mut sig = Signature::new();
    for ty in inputs {
        sig.add_input(ty);
    }
    for ty in outputs {
        sig.add_output(ty);
    }
    if let Some(ty) = ret {
        sig.set_return_type(ty);
    }
    sig
}

fn eclass_id(class: EClassRef) -> u32 {
    class.0.rep()
}

fn parse_usize(term: &Term) -> Result<usize> {
    let atom = term_atom(term)?;
    atom.parse::<usize>().context("invalid usize")
}

fn parse_bigint(value: String) -> Result<BigInt> {
    value.parse::<BigInt>().context("invalid bigint")
}

fn term_atom(term: &Term) -> Result<String> {
    match term {
        Term::Atom(value) => Ok(value.clone()),
        _ => bail!("expected atom"),
    }
}

fn term_string(term: &Term) -> Result<String> {
    match term {
        Term::Str(value) => Ok(value.clone()),
        Term::Atom(value) => Ok(value.clone()),
        _ => bail!("expected string"),
    }
}

#[derive(Debug, Clone)]
enum Token {
    LParen,
    RParen,
    LBracket,
    RBracket,
    Symbol(String),
    Str(String),
}

fn parse_line(line: &str) -> Result<Term> {
    let tokens = tokenize(line)?;
    let mut idx = 0;
    let term = parse_term(&tokens, &mut idx)?;
    if idx != tokens.len() {
        bail!("unexpected tokens after term");
    }
    Ok(term)
}

fn parse_term(tokens: &[Token], idx: &mut usize) -> Result<Term> {
    let token = tokens.get(*idx).ok_or_else(|| anyhow!("unexpected end"))?;
    match token {
        Token::LParen | Token::LBracket => {
            let closing = matches!(token, Token::LParen)
                .then_some(Token::RParen)
                .unwrap_or(Token::RBracket);
            *idx += 1;
            let mut items = Vec::new();
            while *idx < tokens.len() {
                if matches!(tokens[*idx], Token::RParen) || matches!(tokens[*idx], Token::RBracket)
                {
                    break;
                }
                items.push(parse_term(tokens, idx)?);
            }
            if *idx >= tokens.len() {
                bail!("missing closing delimiter");
            }
            let close = &tokens[*idx];
            if std::mem::discriminant(close) != std::mem::discriminant(&closing) {
                bail!("mismatched closing delimiter");
            }
            *idx += 1;
            Ok(Term::List(items))
        }
        Token::Symbol(value) => {
            *idx += 1;
            Ok(Term::Atom(value.clone()))
        }
        Token::Str(value) => {
            *idx += 1;
            Ok(Term::Str(value.clone()))
        }
        Token::RParen | Token::RBracket => bail!("unexpected closing delimiter"),
    }
}

fn tokenize(line: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&ch) = chars.peek() {
        match ch {
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '[' => {
                chars.next();
                tokens.push(Token::LBracket);
            }
            ']' => {
                chars.next();
                tokens.push(Token::RBracket);
            }
            '"' => {
                chars.next();
                let mut out = String::new();
                while let Some(next) = chars.next() {
                    match next {
                        '"' => break,
                        '\\' => {
                            let escaped = chars.next().ok_or_else(|| anyhow!("invalid escape"))?;
                            match escaped {
                                '"' => out.push('"'),
                                '\\' => out.push('\\'),
                                'n' => out.push('\n'),
                                other => out.push(other),
                            }
                        }
                        other => out.push(other),
                    }
                }
                tokens.push(Token::Str(out));
            }
            ';' => break,
            _ if ch.is_whitespace() => {
                chars.next();
            }
            _ => {
                let mut out = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']') {
                        break;
                    }
                    out.push(c);
                    chars.next();
                }
                tokens.push(Token::Symbol(out));
            }
        }
    }
    Ok(tokens)
}

fn format_term(term: &Term) -> String {
    match term {
        Term::Atom(value) => value.clone(),
        Term::Str(value) => format!("\"{}\"", escape_string(value)),
        Term::List(values) => {
            let rendered = values.iter().map(format_term).collect::<Vec<_>>().join(" ");
            format!("({})", rendered)
        }
    }
}

fn escape_string(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}
