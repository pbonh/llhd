// Copyright (c) 2017-2021 Fabian Schuiki

//! CFG skeleton for e-graph based rewrites.
//!
//! The skeleton preserves control flow and side-effecting operations while the
//! e-graph models pure expressions in between. Phi nodes are represented as
//! block arguments, and per-edge arguments are recorded in the incoming map.

use crate::ir::{Block, Inst, Opcode, Unit, Value};
use std::collections::HashMap;

/// E-graph lookup and extraction hooks required by the skeleton.
pub trait EGraphLookup {
    /// The identifier for an e-class.
    type EClassId: Copy + Eq + std::hash::Hash;

    /// Get the e-class for a value in the IR.
    fn eclass_of_value(&self, value: Value) -> Self::EClassId;

    /// Get the e-class for an instruction, if any.
    fn eclass_of_inst(&self, inst: Inst) -> Option<Self::EClassId>;

    /// Extract a value from the e-graph for a given e-class.
    fn extract_value(&self, eclass: Self::EClassId) -> Value;

    /// Extract an instruction from the e-graph for a given e-class.
    fn extract_inst(&self, eclass: Self::EClassId) -> Option<Inst>;
}

/// A CFG skeleton built from a unit and an external e-graph.
#[derive(Debug, Clone)]
pub struct CfgSkeleton<EClassId> {
    blocks: Vec<SkBlock<EClassId>>,
    block_map: HashMap<Block, usize>,
}

impl<EClassId> CfgSkeleton<EClassId> {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            block_map: HashMap::new(),
        }
    }

    /// Return the blocks in layout order.
    pub fn blocks(&self) -> &[SkBlock<EClassId>] {
        &self.blocks
    }

    /// Return a block by id.
    pub fn block(&self, bb: Block) -> Option<&SkBlock<EClassId>> {
        self.block_map.get(&bb).map(|&idx| &self.blocks[idx])
    }

    /// Return a mutable block by id.
    pub fn block_mut(&mut self, bb: Block) -> Option<&mut SkBlock<EClassId>> {
        self.block_map
            .get(&bb)
            .cloned()
            .map(move |idx| &mut self.blocks[idx])
    }

    /// Detach an effectful statement from the skeleton.
    pub fn detach_effect(&mut self, bb: Block, index: usize) -> bool {
        if let Some(block) = self.block_mut(bb) {
            if let Some(effect) = block.effects.get_mut(index) {
                effect.detached = true;
                return true;
            }
        }
        false
    }
}

/// A block in the CFG skeleton.
#[derive(Debug, Clone)]
pub struct SkBlock<EClassId> {
    /// The block id in the unit.
    pub block: Block,
    /// The block arguments (phi results), in phi order.
    pub args: Vec<EClassId>,
    /// Incoming arguments per predecessor block.
    pub incoming: HashMap<Block, Vec<EClassId>>,
    /// Effectful statements in the block, in layout order.
    pub effects: Vec<SkEffect<EClassId>>,
    /// The terminator statement of the block, if any.
    pub terminator: Option<SkTerminator<EClassId>>,
}

/// A side-effecting statement tracked in the skeleton.
#[derive(Debug, Clone)]
pub struct SkEffect<EClassId> {
    /// The originating instruction.
    pub inst: Inst,
    /// The opcode of the instruction.
    pub opcode: Opcode,
    /// The operand e-classes of the instruction.
    pub args: Vec<EClassId>,
    /// The e-class for the instruction result, if any.
    pub result: Option<EClassId>,
    /// Whether this statement is detached from the skeleton.
    pub detached: bool,
}

/// A control-flow statement tracked in the skeleton.
#[derive(Debug, Clone)]
pub struct SkTerminator<EClassId> {
    /// The originating instruction.
    pub inst: Inst,
    /// The opcode of the instruction.
    pub opcode: Opcode,
    /// The operand e-classes of the instruction.
    pub args: Vec<EClassId>,
    /// The destination blocks of the instruction.
    pub blocks: Vec<Block>,
}

/// Build a CFG skeleton from a unit and an external e-graph.
pub fn cfg_skeleton<E: EGraphLookup>(unit: &Unit, egraph: &E) -> CfgSkeleton<E::EClassId> {
    let mut skeleton = CfgSkeleton::new();
    for bb in unit.blocks() {
        let mut sk_block = SkBlock {
            block: bb,
            args: Vec::new(),
            incoming: HashMap::new(),
            effects: Vec::new(),
            terminator: None,
        };

        for inst in unit.insts(bb) {
            let opcode = unit[inst].opcode();
            if opcode.is_phi() {
                let result = unit.inst_result(inst);
                sk_block.args.push(egraph.eclass_of_value(result));
                let data = &unit[inst];
                for (arg, pred) in data.args().iter().zip(data.blocks().iter()) {
                    sk_block
                        .incoming
                        .entry(*pred)
                        .or_insert_with(Vec::new)
                        .push(egraph.eclass_of_value(*arg));
                }
                continue;
            }

            if opcode.is_control_flow() {
                let args = unit[inst]
                    .args()
                    .iter()
                    .map(|&value| egraph.eclass_of_value(value))
                    .collect();
                let blocks = unit[inst].blocks().iter().cloned().collect();
                sk_block.terminator = Some(SkTerminator {
                    inst,
                    opcode,
                    args,
                    blocks,
                });
                continue;
            }

            if opcode.is_effectful() {
                let args = unit[inst]
                    .args()
                    .iter()
                    .map(|&value| egraph.eclass_of_value(value))
                    .collect();
                let result = if unit.has_result(inst) {
                    Some(egraph.eclass_of_value(unit.inst_result(inst)))
                } else {
                    None
                };
                sk_block.effects.push(SkEffect {
                    inst,
                    opcode,
                    args,
                    result,
                    detached: false,
                });
            }
        }

        debug_assert!(sk_block
            .incoming
            .values()
            .all(|incoming| incoming.len() == sk_block.args.len()));

        let index = skeleton.blocks.len();
        skeleton.block_map.insert(bb, index);
        skeleton.blocks.push(sk_block);
    }
    skeleton
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::parse_module;
    use crate::ir::{Signature, UnitBuilder, UnitData, UnitKind, UnitName};
    use crate::ty::{int_ty, signal_ty};
    use crate::value::{IntValue, TimeValue};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    #[derive(Default)]
    struct TestEGraph {
        next: Cell<u32>,
        values: RefCell<HashMap<Value, u32>>,
        insts: RefCell<HashMap<Inst, u32>>,
    }

    impl TestEGraph {
        fn fresh(&self) -> u32 {
            let id = self.next.get();
            self.next.set(id + 1);
            id
        }

        fn class_for_value(&self, value: Value) -> u32 {
            if let Some(id) = self.values.borrow().get(&value).cloned() {
                return id;
            }
            let id = self.fresh();
            self.values.borrow_mut().insert(value, id);
            id
        }

        fn class_for_inst(&self, inst: Inst) -> u32 {
            if let Some(id) = self.insts.borrow().get(&inst).cloned() {
                return id;
            }
            let id = self.fresh();
            self.insts.borrow_mut().insert(inst, id);
            id
        }
    }

    impl EGraphLookup for TestEGraph {
        type EClassId = u32;

        fn eclass_of_value(&self, value: Value) -> Self::EClassId {
            self.class_for_value(value)
        }

        fn eclass_of_inst(&self, inst: Inst) -> Option<Self::EClassId> {
            Some(self.class_for_inst(inst))
        }

        fn extract_value(&self, _eclass: Self::EClassId) -> Value {
            Value::invalid()
        }

        fn extract_inst(&self, _eclass: Self::EClassId) -> Option<Inst> {
            None
        }
    }

    fn unit_by_name<'a>(module: &'a crate::ir::Module, name: &str) -> Unit<'a> {
        module
            .units()
            .find(|unit| unit.name().get_name() == Some(name))
            .unwrap_or_else(|| panic!("unit {} missing", name))
    }

    fn block_by_name(unit: Unit, name: &str) -> Block {
        unit.blocks()
            .find(|&bb| unit.get_block_name(bb) == Some(name))
            .unwrap_or_else(|| panic!("block {} missing", name))
    }

    fn value_by_name(unit: Unit, name: &str) -> Value {
        unit.all_insts()
            .find_map(|inst| {
                if !unit.has_result(inst) {
                    return None;
                }
                let value = unit.inst_result(inst);
                (unit.get_name(value) == Some(name)).then_some(value)
            })
            .unwrap_or_else(|| panic!("value {} missing", name))
    }

    #[test]
    fn skeleton_diamond_phi_merge() {
        let mut sig = Signature::new();
        sig.set_return_type(int_ty(32));
        let mut data = UnitData::new(UnitKind::Function, UnitName::local("diamond"), sig);
        let mut builder = UnitBuilder::new_anonymous(&mut data);

        let entry = builder.block();
        let then_bb = builder.block();
        let else_bb = builder.block();
        let merge = builder.block();

        builder.append_to(entry);
        let cond = builder.ins().const_int(IntValue::from_usize(1, 1));
        builder.ins().br_cond(cond, then_bb, else_bb);

        builder.append_to(then_bb);
        let then_val = builder.ins().const_int(IntValue::from_usize(32, 10));
        let ptr = builder.ins().var(then_val);
        builder.ins().st(ptr, then_val);
        builder.ins().br(merge);

        builder.append_to(else_bb);
        let else_val = builder.ins().const_int(IntValue::from_usize(32, 20));
        builder.ins().br(merge);

        builder.append_to(merge);
        let merged = builder
            .ins()
            .phi(vec![then_val, else_val], vec![then_bb, else_bb]);
        builder.ins().ret_value(merged);

        let unit = builder.finish();
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        assert_eq!(skeleton.blocks().len(), 4);

        let merge_block = skeleton.block(merge).expect("merge block missing");
        assert_eq!(merge_block.args.len(), 1);
        assert_eq!(
            merge_block.incoming.get(&then_bb).unwrap(),
            &vec![egraph.eclass_of_value(then_val)]
        );
        assert_eq!(
            merge_block.incoming.get(&else_bb).unwrap(),
            &vec![egraph.eclass_of_value(else_val)]
        );

        let entry_block = skeleton.block(entry).expect("entry block missing");
        let term = entry_block.terminator.as_ref().expect("entry terminator");
        assert_eq!(term.opcode, Opcode::BrCond);
        assert_eq!(term.blocks, vec![then_bb, else_bb]);

        let then_block = skeleton.block(then_bb).expect("then block missing");
        assert_eq!(then_block.effects.len(), 2);
        assert_eq!(then_block.effects[0].opcode, Opcode::Var);
        assert_eq!(then_block.effects[1].opcode, Opcode::St);
    }

    #[test]
    fn skeleton_diamond_phi_merge_from_string() {
        let module = parse_module(
            r#"
            func @diamond () i32 {
            entry:
                %cond = const i1 1
                br %cond, %then, %else
            then:
                %then_val = const i32 10
                %ptr = var i32 %then_val
                st i32* %ptr, %then_val
                br %merge
            else:
                %else_val = const i32 20
                br %merge
            merge:
                %merged = phi i32 [%then_val, %then], [%else_val, %else]
                ret i32 %merged
            }
            "#,
        )
        .unwrap();

        let unit = unit_by_name(&module, "diamond");
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        let entry = block_by_name(unit, "entry");
        let then_bb = block_by_name(unit, "then");
        let else_bb = block_by_name(unit, "else");
        let merge = block_by_name(unit, "merge");
        let then_val = value_by_name(unit, "then_val");
        let else_val = value_by_name(unit, "else_val");

        assert_eq!(skeleton.blocks().len(), 4);

        let merge_block = skeleton.block(merge).expect("merge block missing");
        assert_eq!(merge_block.args.len(), 1);
        assert_eq!(
            merge_block.incoming.get(&then_bb).unwrap(),
            &vec![egraph.eclass_of_value(then_val)]
        );
        assert_eq!(
            merge_block.incoming.get(&else_bb).unwrap(),
            &vec![egraph.eclass_of_value(else_val)]
        );

        let entry_block = skeleton.block(entry).expect("entry block missing");
        let term = entry_block.terminator.as_ref().expect("entry terminator");
        assert_eq!(term.opcode, Opcode::BrCond);
        assert_eq!(term.blocks, vec![then_bb, else_bb]);

        let then_block = skeleton.block(then_bb).expect("then block missing");
        assert_eq!(then_block.effects.len(), 2);
        assert_eq!(then_block.effects[0].opcode, Opcode::Var);
        assert_eq!(then_block.effects[1].opcode, Opcode::St);
    }

    #[test]
    fn skeleton_loop_with_phi_backedge() {
        let mut sig = Signature::new();
        sig.set_return_type(int_ty(32));
        let mut data = UnitData::new(UnitKind::Function, UnitName::local("loop"), sig);
        let mut builder = UnitBuilder::new_anonymous(&mut data);

        let entry = builder.block();
        let header = builder.block();
        let body = builder.block();
        let exit = builder.block();

        builder.append_to(entry);
        let init = builder.ins().const_int(IntValue::from_usize(32, 0));
        builder.ins().br(header);

        builder.append_to(body);
        let body_val = builder.ins().const_int(IntValue::from_usize(32, 1));
        builder.ins().br(header);

        builder.append_to(header);
        let phi = builder.ins().phi(vec![init, body_val], vec![entry, body]);
        let cond = builder.ins().const_int(IntValue::from_usize(1, 0));
        builder.ins().br_cond(cond, body, exit);

        builder.append_to(exit);
        builder.ins().ret_value(phi);

        let unit = builder.finish();
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        let header_block = skeleton.block(header).expect("header block missing");
        assert_eq!(header_block.args.len(), 1);
        assert_eq!(
            header_block.incoming.get(&entry).unwrap(),
            &vec![egraph.eclass_of_value(init)]
        );
        assert_eq!(
            header_block.incoming.get(&body).unwrap(),
            &vec![egraph.eclass_of_value(body_val)]
        );
        assert_eq!(
            header_block.terminator.as_ref().unwrap().opcode,
            Opcode::BrCond
        );
    }

    #[test]
    fn skeleton_loop_with_phi_backedge_from_string() {
        let module = parse_module(
            r#"
            func @loop () i32 {
            entry:
                %init = const i32 0
                br %header
            body:
                %body_val = const i32 1
                br %header
            header:
                %phi = phi i32 [%init, %entry], [%body_val, %body]
                %cond = const i1 0
                br %cond, %body, %exit
            exit:
                ret i32 %phi
            }
            "#,
        )
        .unwrap();

        let unit = unit_by_name(&module, "loop");
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        let entry = block_by_name(unit, "entry");
        let header = block_by_name(unit, "header");
        let body = block_by_name(unit, "body");
        let init = value_by_name(unit, "init");
        let body_val = value_by_name(unit, "body_val");

        let header_block = skeleton.block(header).expect("header block missing");
        assert_eq!(header_block.args.len(), 1);
        assert_eq!(
            header_block.incoming.get(&entry).unwrap(),
            &vec![egraph.eclass_of_value(init)]
        );
        assert_eq!(
            header_block.incoming.get(&body).unwrap(),
            &vec![egraph.eclass_of_value(body_val)]
        );
        assert_eq!(
            header_block.terminator.as_ref().unwrap().opcode,
            Opcode::BrCond
        );
    }

    #[test]
    fn skeleton_process_wait_and_effects() {
        let mut sig = Signature::new();
        sig.add_input(signal_ty(int_ty(1)));
        let mut data = UnitData::new(UnitKind::Process, UnitName::local("proc"), sig);
        let mut builder = UnitBuilder::new_anonymous(&mut data);

        let entry = builder.block();
        let next = builder.block();

        builder.append_to(entry);
        let sig_in = builder.input_arg(0);
        let value = builder.ins().const_int(IntValue::from_usize(1, 1));
        let delay = builder.ins().const_time(TimeValue::zero());
        builder.ins().drv(sig_in, value, delay);
        builder.ins().wait(next, vec![sig_in]);

        builder.append_to(next);
        builder.ins().halt();

        let unit = builder.finish();
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        let entry_block = skeleton.block(entry).expect("entry block missing");
        assert_eq!(entry_block.effects.len(), 1);
        assert_eq!(entry_block.effects[0].opcode, Opcode::Drv);
        assert_eq!(
            entry_block.terminator.as_ref().unwrap().opcode,
            Opcode::Wait
        );
    }

    #[test]
    fn skeleton_process_wait_and_effects_from_string() {
        let module = parse_module(
            r#"
            proc @proc (i1$ %sig_in) -> () {
            entry:
                %value = const i1 1
                %delay = const time 0s
                drv i1$ %sig_in, %value, %delay
                wait %next, %sig_in
            next:
                halt
            }
            "#,
        )
        .unwrap();

        let unit = unit_by_name(&module, "proc");
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        let entry = block_by_name(unit, "entry");
        let entry_block = skeleton.block(entry).expect("entry block missing");
        assert_eq!(entry_block.effects.len(), 1);
        assert_eq!(entry_block.effects[0].opcode, Opcode::Drv);
        assert_eq!(
            entry_block.terminator.as_ref().unwrap().opcode,
            Opcode::Wait
        );
    }

    #[test]
    fn skeleton_call_and_inst_effect_classification() {
        let mut sig = Signature::new();
        sig.add_input(signal_ty(int_ty(1)));
        sig.add_output(signal_ty(int_ty(1)));
        let mut data = UnitData::new(UnitKind::Entity, UnitName::local("ent"), sig);
        let mut builder = UnitBuilder::new_anonymous(&mut data);

        let mut call_sig = Signature::new();
        call_sig.set_return_type(int_ty(1));
        let call_ext = builder.add_extern(UnitName::local("callee"), call_sig);

        let mut inst_sig = Signature::new();
        inst_sig.add_input(signal_ty(int_ty(1)));
        inst_sig.add_output(signal_ty(int_ty(1)));
        let inst_ext = builder.add_extern(UnitName::local("child"), inst_sig);

        let input = builder.input_arg(0);
        let output = builder.output_arg(0);
        builder.ins().inst(inst_ext, vec![input], vec![output]);
        builder.ins().call(call_ext, vec![]);

        let unit = builder.finish();
        let egraph = TestEGraph::default();
        let mut skeleton = cfg_skeleton(&unit, &egraph);

        let block = skeleton.blocks()[0].block;
        let block_data = skeleton.block(block).expect("entity block missing");
        assert_eq!(block_data.effects.len(), 1);
        assert_eq!(block_data.effects[0].opcode, Opcode::Call);

        assert!(skeleton.detach_effect(block, 0));
        let block_data = skeleton.block(block).unwrap();
        assert!(block_data.effects[0].detached);
    }

    #[test]
    fn skeleton_call_and_inst_effect_classification_from_string() {
        let module = parse_module(
            r#"
            declare @callee () i1

            entity @child (i1$ %input) -> (i1$ %output) {
            }

            entity @ent (i1$ %input) -> (i1$ %output) {
                inst @child (i1$ %input) -> (i1$ %output)
                %call = call i1 @callee ()
            }
            "#,
        )
        .unwrap();

        let unit = unit_by_name(&module, "ent");
        let egraph = TestEGraph::default();
        let mut skeleton = cfg_skeleton(&unit, &egraph);

        let block = skeleton.blocks()[0].block;
        let block_data = skeleton.block(block).expect("entity block missing");
        assert_eq!(block_data.effects.len(), 1);
        assert_eq!(block_data.effects[0].opcode, Opcode::Call);

        assert!(skeleton.detach_effect(block, 0));
        let block_data = skeleton.block(block).unwrap();
        assert!(block_data.effects[0].detached);
    }

    #[test]
    fn skeleton_mixed_wait_time_and_br_cond() {
        let mut sig = Signature::new();
        sig.add_input(signal_ty(int_ty(1)));
        let mut data = UnitData::new(UnitKind::Process, UnitName::local("mix"), sig);
        let mut builder = UnitBuilder::new_anonymous(&mut data);

        let entry = builder.block();
        let branch = builder.block();
        let wait_block = builder.block();
        let exit = builder.block();

        builder.append_to(entry);
        let cond = builder.ins().const_int(IntValue::from_usize(1, 1));
        builder.ins().br_cond(cond, branch, wait_block);

        builder.append_to(branch);
        builder.ins().br(exit);

        builder.append_to(wait_block);
        let delay = builder.ins().const_time(TimeValue::zero());
        let signal = builder.input_arg(0);
        builder.ins().wait_time(exit, delay, vec![signal]);

        builder.append_to(exit);
        builder.ins().halt();

        let unit = builder.finish();
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        let entry_block = skeleton.block(entry).expect("entry block missing");
        assert_eq!(
            entry_block.terminator.as_ref().unwrap().opcode,
            Opcode::BrCond
        );
        assert_eq!(
            entry_block.terminator.as_ref().unwrap().blocks,
            vec![branch, wait_block]
        );

        let wait = skeleton.block(wait_block).expect("wait block missing");
        assert_eq!(wait.terminator.as_ref().unwrap().opcode, Opcode::WaitTime);
        assert_eq!(wait.terminator.as_ref().unwrap().blocks, vec![exit]);
    }

    #[test]
    fn skeleton_mixed_wait_time_and_br_cond_from_string() {
        let module = parse_module(
            r#"
            proc @mix (i1$ %signal) -> () {
            entry:
                %cond = const i1 1
                br %cond, %branch, %wait_block
            branch:
                br %exit
            wait_block:
                %delay = const time 0s
                wait %exit for %delay, %signal
            exit:
                halt
            }
            "#,
        )
        .unwrap();

        let unit = unit_by_name(&module, "mix");
        let egraph = TestEGraph::default();
        let skeleton = cfg_skeleton(&unit, &egraph);

        let entry = block_by_name(unit, "entry");
        let branch = block_by_name(unit, "branch");
        let wait_block = block_by_name(unit, "wait_block");
        let exit = block_by_name(unit, "exit");

        let entry_block = skeleton.block(entry).expect("entry block missing");
        assert_eq!(
            entry_block.terminator.as_ref().unwrap().opcode,
            Opcode::BrCond
        );
        assert_eq!(
            entry_block.terminator.as_ref().unwrap().blocks,
            vec![branch, wait_block]
        );

        let wait = skeleton.block(wait_block).expect("wait block missing");
        assert_eq!(wait.terminator.as_ref().unwrap().opcode, Opcode::WaitTime);
        assert_eq!(wait.terminator.as_ref().unwrap().blocks, vec![exit]);
    }
}
