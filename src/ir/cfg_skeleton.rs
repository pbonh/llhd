use crate::ir::{is_pure_opcode, EClassRef, UnitEGraph};
use crate::ir::{Block, Inst, InstData, Opcode, Unit, Value};
use anyhow::Result;
use std::collections::HashMap;
use std::fmt::Write;

/// A CFG skeleton for an LLHD unit, capturing control flow and side effects.
#[derive(Debug, Clone, Default)]
pub struct CfgSkeleton {
    /// Blocks in traversal order with reduced IR details.
    pub blocks: Vec<SkeletonBlock>,
}

/// A block entry with arguments, statements, and terminator.
#[derive(Debug, Clone)]
pub struct SkeletonBlock {
    /// The original block id.
    pub block: Block,
    /// Block arguments mapped to e-classes.
    pub args: Vec<BlockArg>,
    /// Effectful statements in this block.
    pub stmts: Vec<SkeletonStmt>,
    /// Terminator for the block, if any.
    pub terminator: Option<SkeletonTerminator>,
}

/// A single block argument and its e-class.
#[derive(Debug, Clone)]
pub struct BlockArg {
    /// The value for the argument.
    pub value: Value,
    /// The e-class representing the argument.
    pub class: EClassRef,
}

/// A reduced statement capturing side effects.
#[derive(Debug, Clone)]
pub enum SkeletonStmt {
    /// An effectful instruction with optional result class.
    Effect {
        /// The instruction id.
        inst: Inst,
        /// The opcode for the instruction.
        opcode: Opcode,
        /// Argument e-classes.
        args: Vec<EClassRef>,
        /// Result e-class, if any.
        result: Option<EClassRef>,
    },
}

/// A reduced terminator capturing control flow.
#[derive(Debug, Clone)]
pub enum SkeletonTerminator {
    /// Unconditional branch.
    Br {
        /// The branch instruction.
        inst: Inst,
        /// Target block.
        target: Block,
        /// Argument e-classes passed to the target.
        args: Vec<EClassRef>,
    },
    /// Conditional branch.
    BrCond {
        /// The branch instruction.
        inst: Inst,
        /// Condition e-class.
        cond: EClassRef,
        /// Then target block.
        then_target: Block,
        /// Argument e-classes for the then edge.
        then_args: Vec<EClassRef>,
        /// Else target block.
        else_target: Block,
        /// Argument e-classes for the else edge.
        else_args: Vec<EClassRef>,
    },
    /// Wait terminator with target block.
    Wait {
        /// The wait instruction.
        inst: Inst,
        /// Target block.
        target: Block,
        /// Argument e-classes passed to the target.
        args: Vec<EClassRef>,
    },
    /// Wait-for-time terminator with target block.
    WaitTime {
        /// The waittime instruction.
        inst: Inst,
        /// Time e-class.
        time: EClassRef,
        /// Target block.
        target: Block,
        /// Argument e-classes passed to the target.
        args: Vec<EClassRef>,
    },
    /// Return terminator with no value.
    Ret {
        /// The return instruction.
        inst: Inst,
    },
    /// Return terminator with a value.
    RetValue {
        /// The return instruction.
        inst: Inst,
        /// Returned value e-class.
        value: EClassRef,
    },
    /// Halt terminator.
    Halt {
        /// The halt instruction.
        inst: Inst,
    },
}

impl CfgSkeleton {
    /// Build a CFG skeleton from a unit and its e-graph mapping.
    pub fn build_from_unit(unit: &Unit<'_>, egraph: &mut UnitEGraph) -> Result<Self> {
        let phi_map = collect_phi_info(unit, egraph)?;
        let mut blocks = Vec::new();

        for bb in unit.blocks() {
            let phi_infos = phi_map.get(&bb).cloned().unwrap_or_default();
            let args = phi_infos
                .iter()
                .map(|phi| BlockArg {
                    value: phi.value,
                    class: phi.class,
                })
                .collect::<Vec<_>>();

            let mut stmts = Vec::new();
            let mut terminator = None;

            for inst in unit.insts(bb) {
                let opcode = unit[inst].opcode();
                if opcode == Opcode::Phi {
                    continue;
                }

                if opcode.is_terminator() {
                    terminator = Some(build_terminator(unit, egraph, inst, &phi_map, bb)?);
                    continue;
                }

                if !is_pure_opcode(opcode) {
                    let args = unit[inst]
                        .args()
                        .iter()
                        .map(|&arg| egraph.ensure_value_ref(unit, arg))
                        .collect::<Result<Vec<_>, _>>()?;
                    let result = unit
                        .get_inst_result(inst)
                        .map(|value| egraph.ensure_value_ref(unit, value))
                        .transpose()?;
                    stmts.push(SkeletonStmt::Effect {
                        inst,
                        opcode,
                        args,
                        result,
                    });
                }
            }

            blocks.push(SkeletonBlock {
                block: bb,
                args,
                stmts,
                terminator,
            });
        }

        Ok(Self { blocks })
    }

    /// Dump the CFG skeleton in human-readable form.
    pub fn dump(&self, unit: &Unit<'_>) -> String {
        let mut out = String::new();
        for block in &self.blocks {
            let _ = writeln!(out, "{}:", block.block.dump(unit));
            if !block.args.is_empty() {
                let args = block
                    .args
                    .iter()
                    .map(|arg| format!("{} => {}", arg.value.dump(unit), arg.class))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(out, "  args: {}", args);
            }
            for stmt in &block.stmts {
                match stmt {
                    SkeletonStmt::Effect {
                        inst,
                        opcode,
                        args,
                        result,
                    } => {
                        let args = args
                            .iter()
                            .map(|arg| arg.to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        if let Some(result) = result {
                            let _ = writeln!(
                                out,
                                "  {} [{}] ({}) => {}",
                                inst.dump(unit),
                                opcode,
                                args,
                                result
                            );
                        } else {
                            let _ = writeln!(out, "  {} [{}] ({})", inst.dump(unit), opcode, args);
                        }
                    }
                }
            }
            if let Some(term) = &block.terminator {
                let _ = writeln!(out, "  term: {}", term.dump(unit));
            }
        }
        out
    }
}

impl SkeletonTerminator {
    fn dump(&self, unit: &Unit<'_>) -> String {
        match self {
            SkeletonTerminator::Br { inst, target, args } => format!(
                "{} br {} ({})",
                inst.dump(unit),
                target.dump(unit),
                format_args(args)
            ),
            SkeletonTerminator::BrCond {
                inst,
                cond,
                then_target,
                then_args,
                else_target,
                else_args,
            } => format!(
                "{} brcond {} ? {}({}) : {}({})",
                inst.dump(unit),
                cond,
                then_target.dump(unit),
                format_args(then_args),
                else_target.dump(unit),
                format_args(else_args)
            ),
            SkeletonTerminator::Wait { inst, target, args } => format!(
                "{} wait {} ({})",
                inst.dump(unit),
                target.dump(unit),
                format_args(args)
            ),
            SkeletonTerminator::WaitTime {
                inst,
                time,
                target,
                args,
            } => format!(
                "{} waittime {} {} ({})",
                inst.dump(unit),
                time,
                target.dump(unit),
                format_args(args)
            ),
            SkeletonTerminator::Ret { inst } => format!("{} ret", inst.dump(unit)),
            SkeletonTerminator::RetValue { inst, value } => {
                format!("{} ret {}", inst.dump(unit), value)
            }
            SkeletonTerminator::Halt { inst } => format!("{} halt", inst.dump(unit)),
        }
    }
}

#[derive(Debug, Clone)]
struct PhiInfo {
    value: Value,
    class: EClassRef,
    incoming: HashMap<Block, EClassRef>,
}

fn collect_phi_info(
    unit: &Unit<'_>,
    egraph: &mut UnitEGraph,
) -> Result<HashMap<Block, Vec<PhiInfo>>> {
    let mut out: HashMap<Block, Vec<PhiInfo>> = HashMap::new();
    for bb in unit.blocks() {
        for inst in unit.insts(bb) {
            if unit[inst].opcode() != Opcode::Phi {
                continue;
            }
            let (args, bbs) = match &unit[inst] {
                InstData::Phi { args, bbs, .. } => (args, bbs),
                _ => continue,
            };
            let value = match unit.get_inst_result(inst) {
                Some(value) => value,
                None => continue,
            };
            let class = egraph.ensure_value_ref(unit, value)?;
            let mut incoming = HashMap::new();
            for (&arg, &pred) in args.iter().zip(bbs.iter()) {
                let arg_class = egraph.ensure_value_ref(unit, arg)?;
                incoming.insert(pred, arg_class);
            }
            out.entry(bb).or_default().push(PhiInfo {
                value,
                class,
                incoming,
            });
        }
    }
    Ok(out)
}

fn build_terminator(
    unit: &Unit<'_>,
    egraph: &mut UnitEGraph,
    inst: Inst,
    phi_map: &HashMap<Block, Vec<PhiInfo>>,
    pred: Block,
) -> Result<SkeletonTerminator> {
    let data = &unit[inst];
    let opcode = data.opcode();
    match data {
        InstData::Jump { bbs, .. } if opcode == Opcode::Br => Ok(SkeletonTerminator::Br {
            inst,
            target: bbs[0],
            args: phi_args_for_target(phi_map, bbs[0], pred),
        }),
        InstData::Branch { args, bbs, .. } if opcode == Opcode::BrCond => {
            let cond = egraph.ensure_value_ref(unit, args[0])?;
            Ok(SkeletonTerminator::BrCond {
                inst,
                cond,
                then_target: bbs[0],
                then_args: phi_args_for_target(phi_map, bbs[0], pred),
                else_target: bbs[1],
                else_args: phi_args_for_target(phi_map, bbs[1], pred),
            })
        }
        InstData::Wait { bbs, args, .. } if opcode == Opcode::Wait => {
            let args = args
                .iter()
                .map(|&arg| egraph.ensure_value_ref(unit, arg))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(SkeletonTerminator::Wait {
                inst,
                target: bbs[0],
                args,
            })
        }
        InstData::Wait { bbs, args, .. } if opcode == Opcode::WaitTime => {
            let mut args_iter = args.iter();
            let time = args_iter.next().copied().unwrap_or_else(Value::invalid);
            let time = egraph.ensure_value_ref(unit, time)?;
            let rest = args_iter
                .map(|&arg| egraph.ensure_value_ref(unit, arg))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(SkeletonTerminator::WaitTime {
                inst,
                time,
                target: bbs[0],
                args: rest,
            })
        }
        InstData::Nullary { .. } if opcode == Opcode::Ret => Ok(SkeletonTerminator::Ret { inst }),
        InstData::Unary { args, .. } if opcode == Opcode::RetValue => {
            let value = egraph.ensure_value_ref(unit, args[0])?;
            Ok(SkeletonTerminator::RetValue { inst, value })
        }
        InstData::Nullary { .. } if opcode == Opcode::Halt => Ok(SkeletonTerminator::Halt { inst }),
        _ => Ok(SkeletonTerminator::Halt { inst }),
    }
}

fn phi_args_for_target(
    phi_map: &HashMap<Block, Vec<PhiInfo>>,
    target: Block,
    pred: Block,
) -> Vec<EClassRef> {
    phi_map
        .get(&target)
        .map(|phis| {
            phis.iter()
                .map(|phi| phi.incoming.get(&pred).copied().unwrap_or(phi.class))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn format_args(args: &[EClassRef]) -> String {
    args.iter()
        .map(|arg| arg.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
