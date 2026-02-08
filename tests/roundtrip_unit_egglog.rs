use llhd::assembly::parse_module_unchecked;
use llhd::ir::prelude::*;
use llhd::ir::{unit_from_egglog_program, unit_to_egglog_program};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn collect_llhd_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_llhd_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("llhd") {
            out.push(path);
        }
    }
}

fn insts_for_compare(unit: Unit<'_>, block: Block) -> Vec<Inst> {
    unit.insts(block)
        .filter(|&inst| !(unit.is_entity() && unit[inst].opcode().is_terminator()))
        .collect()
}

fn block_successors(unit: Unit<'_>, block: Block) -> Vec<Block> {
    let Some(term) = unit.last_inst(block) else {
        return Vec::new();
    };
    unit[term].blocks().to_vec()
}

fn inst_signature(unit: Unit<'_>, inst: Inst) -> String {
    let data = &unit[inst];
    let mut out = String::new();
    out.push_str(&format!("{:?}", data.opcode()));
    out.push_str(&format!("|ty:{}", unit.inst_type(inst)));
    out.push_str(&format!("|argc:{}", data.args().len()));
    out.push_str("|imms:");
    for imm in data.imms() {
        out.push_str(&format!("{}", imm));
        out.push(',');
    }
    if data.opcode() == Opcode::ConstInt {
        if let Some(imm) = data.get_const_int() {
            out.push_str(&format!("|cint:{}", imm.value));
        }
    }
    if data.opcode() == Opcode::ConstTime {
        if let Some(imm) = data.get_const_time() {
            out.push_str(&format!("|ctime:{}", imm));
        }
    }
    if let Some(ext) = data.get_ext_unit() {
        out.push_str(&format!("|ext:{}:{}", unit[ext].name, unit[ext].sig));
    }
    if data.opcode() == Opcode::Reg {
        out.push_str("|reg:");
        for trigger in data.triggers() {
            out.push_str(&format!("{:?}", trigger.mode));
            out.push(':');
            out.push_str(if trigger.gate.is_some() {
                "gate"
            } else {
                "nogate"
            });
            out.push(',');
        }
    }
    out
}

fn block_signature(unit: Unit<'_>, block: Block) -> String {
    let mut parts = Vec::new();
    for inst in insts_for_compare(unit, block) {
        parts.push(inst_signature(unit, inst));
    }
    parts.join("|")
}

fn block_order(unit: Unit<'_>) -> Vec<Block> {
    let mut visited = HashSet::new();
    let mut postorder = Vec::new();

    if let Some(entry) = unit.first_block() {
        let mut stack = vec![(entry, 0usize)];
        visited.insert(entry);
        while let Some((block, idx)) = stack.pop() {
            let succs = block_successors(unit, block);
            if idx < succs.len() {
                stack.push((block, idx + 1));
                let succ = succs[idx];
                if visited.insert(succ) {
                    stack.push((succ, 0));
                }
            } else {
                postorder.push(block);
            }
        }
    }

    postorder.reverse();

    let mut unreachable: Vec<_> = unit.blocks().filter(|b| !visited.contains(b)).collect();
    unreachable.sort_by_key(|block| block_signature(unit, *block));
    postorder.extend(unreachable);
    postorder
}

fn units_equivalent(left: Unit<'_>, right: Unit<'_>) -> Result<(), String> {
    if left.kind() != right.kind() {
        return Err(format!(
            "unit kind mismatch: left {:?}, right {:?}",
            left.kind(),
            right.kind()
        ));
    }
    if left.sig() != right.sig() {
        return Err("unit signature mismatch".to_string());
    }

    let left_blocks = block_order(left);
    let right_blocks = block_order(right);

    if left_blocks.len() != right_blocks.len() {
        return Err("block count mismatch".to_string());
    }

    let mut block_map: HashMap<Block, Block> = HashMap::new();
    for (left_block, right_block) in left_blocks.iter().zip(right_blocks.iter()) {
        block_map.insert(*left_block, *right_block);
    }

    let mut inst_pairs = Vec::new();
    for (left_block, right_block) in left_blocks.into_iter().zip(right_blocks.into_iter()) {
        let left_insts = insts_for_compare(left, left_block);
        let right_insts = insts_for_compare(right, right_block);
        if left_insts.len() != right_insts.len() {
            return Err("instruction count mismatch in block".to_string());
        }
        inst_pairs.extend(left_insts.into_iter().zip(right_insts.into_iter()));
    }

    let left_args: Vec<_> = left.args().collect();
    let right_args: Vec<_> = right.args().collect();
    if left_args.len() != right_args.len() {
        return Err("argument count mismatch".to_string());
    }

    let mut value_map: HashMap<Value, Value> = HashMap::new();
    for (left_value, right_value) in left_args.into_iter().zip(right_args.into_iter()) {
        value_map.insert(left_value, right_value);
    }

    for (left_inst, right_inst) in &inst_pairs {
        let left_has_result = left.has_result(*left_inst);
        let right_has_result = right.has_result(*right_inst);
        if left_has_result != right_has_result {
            return Err("result presence mismatch".to_string());
        }
        if left_has_result {
            value_map.insert(left.inst_result(*left_inst), right.inst_result(*right_inst));
        }
    }

    for (left_value, right_value) in value_map.iter() {
        if left_value.is_invalid() {
            continue;
        }
        if left.value_type(*left_value) != right.value_type(*right_value) {
            return Err("value type mismatch".to_string());
        }
    }

    for (left_inst, right_inst) in inst_pairs {
        let left_data = &left[left_inst];
        let right_data = &right[right_inst];

        if left_data.opcode() != right_data.opcode() {
            return Err(format!(
                "opcode mismatch: left {:?}, right {:?}",
                left_data.opcode(),
                right_data.opcode()
            ));
        }

        if left.inst_type(left_inst) != right.inst_type(right_inst) {
            return Err("instruction type mismatch".to_string());
        }

        let left_args = left_data.args();
        let right_args = right_data.args();
        if left_args.len() != right_args.len() {
            return Err("argument length mismatch".to_string());
        }
        for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
            if left_arg.is_invalid() {
                if !right_arg.is_invalid() {
                    return Err("invalid argument mismatch".to_string());
                }
                continue;
            }
            let mapped = value_map
                .get(left_arg)
                .ok_or_else(|| "missing value mapping".to_string())?;
            if mapped != right_arg {
                return Err("argument mapping mismatch".to_string());
            }
        }

        if left_data.imms() != right_data.imms() {
            return Err("immediate mismatch".to_string());
        }

        let left_blocks = left_data.blocks();
        let right_blocks = right_data.blocks();
        if left_blocks.len() != right_blocks.len() {
            return Err("block operand length mismatch".to_string());
        }
        for (left_block, right_block) in left_blocks.iter().zip(right_blocks.iter()) {
            let mapped = block_map
                .get(left_block)
                .ok_or_else(|| "missing block mapping".to_string())?;
            if mapped != right_block {
                return Err("block mapping mismatch".to_string());
            }
        }

        if left_data.opcode() == Opcode::ConstInt {
            if left_data.get_const_int() != right_data.get_const_int() {
                return Err("const int mismatch".to_string());
            }
        }
        if left_data.opcode() == Opcode::ConstTime {
            if left_data.get_const_time() != right_data.get_const_time() {
                return Err("const time mismatch".to_string());
            }
        }

        match (left_data.get_ext_unit(), right_data.get_ext_unit()) {
            (Some(left_ext), Some(right_ext)) => {
                if left[left_ext].name != right[right_ext].name
                    || left[left_ext].sig != right[right_ext].sig
                {
                    return Err("external unit mismatch".to_string());
                }
            }
            (None, None) => {}
            _ => return Err("external unit presence mismatch".to_string()),
        }

        if left_data.opcode() == Opcode::Reg {
            let left_triggers: Vec<_> = left_data.triggers().collect();
            let right_triggers: Vec<_> = right_data.triggers().collect();
            if left_triggers.len() != right_triggers.len() {
                return Err("register trigger count mismatch".to_string());
            }
            for (left_trigger, right_trigger) in left_triggers.into_iter().zip(right_triggers) {
                if left_trigger.mode != right_trigger.mode {
                    return Err("register mode mismatch".to_string());
                }
                for (left_value, right_value) in [
                    (left_trigger.data, right_trigger.data),
                    (left_trigger.trigger, right_trigger.trigger),
                ] {
                    if left_value.is_invalid() {
                        if !right_value.is_invalid() {
                            return Err("register invalid value mismatch".to_string());
                        }
                        continue;
                    }
                    let mapped = value_map
                        .get(&left_value)
                        .ok_or_else(|| "missing reg value mapping".to_string())?;
                    if mapped != &right_value {
                        return Err("register value mismatch".to_string());
                    }
                }
                match (left_trigger.gate, right_trigger.gate) {
                    (None, None) => {}
                    (Some(left_gate), Some(right_gate)) => {
                        let mapped = value_map
                            .get(&left_gate)
                            .ok_or_else(|| "missing reg gate mapping".to_string())?;
                        if mapped != &right_gate {
                            return Err("register gate mismatch".to_string());
                        }
                    }
                    _ => return Err("register gate presence mismatch".to_string()),
                }
            }
        }
    }

    Ok(())
}

#[test]
fn roundtrip_unit_egglog() {
    let mut files = Vec::new();
    collect_llhd_files(Path::new("tests"), &mut files);
    files.sort();
    let allow_large = env::var("LLHD_ROUNDTRIP_LARGE").ok().as_deref() == Some("1");
    let large_limit = 256 * 1024;

    files.par_iter().for_each(|path| {
        if !allow_large {
            if let Ok(meta) = fs::metadata(path) {
                if meta.len() > large_limit {
                    return;
                }
            }
        }
        let contents = fs::read_to_string(path).expect("read llhd test file");
        if contents.contains("; FAIL") {
            return;
        }

        let module = parse_module_unchecked(&contents)
            .unwrap_or_else(|err| panic!("parse failed for {}: {}", path.display(), err));

        let programs: Vec<_> = module
            .units()
            .map(|unit| unit_to_egglog_program(&unit).expect("egglog program"))
            .collect();

        let mut rebuilt = Module::new();
        for decl in module.decls() {
            let data = module[decl].clone();
            rebuilt.add_decl(data);
        }
        for program in programs {
            let data = unit_from_egglog_program(&program).unwrap_or_else(|err| {
                panic!("egglog parse failed for {}: {}", path.display(), err)
            });
            rebuilt.add_unit(data);
        }

        let mut right_units: Vec<_> = rebuilt.units().collect();
        for left_unit in module.units() {
            let position = right_units.iter().position(|right_unit| {
                left_unit.kind() == right_unit.kind()
                    && left_unit.sig() == right_unit.sig()
                    && left_unit.name() == right_unit.name()
            });
            let Some(index) = position else {
                panic!("unit not found after roundtrip for {}", path.display());
            };
            let right_unit = right_units.swap_remove(index);
            if let Err(err) = units_equivalent(left_unit, right_unit) {
                panic!(
                    "unit mismatch after roundtrip for {}: {}",
                    path.display(),
                    err
                );
            }
        }

        if !right_units.is_empty() {
            panic!("extra units after roundtrip for {}", path.display());
        }

        let mut right_decls: Vec<_> = rebuilt.decls().collect();
        for left_decl in module.decls() {
            let position = right_decls.iter().position(|right_decl| {
                module[left_decl].name == rebuilt[*right_decl].name
                    && module[left_decl].sig == rebuilt[*right_decl].sig
            });
            let Some(index) = position else {
                panic!("decl not found after roundtrip for {}", path.display());
            };
            right_decls.swap_remove(index);
        }
        if !right_decls.is_empty() {
            panic!("extra decls after roundtrip for {}", path.display());
        }
    });
}
