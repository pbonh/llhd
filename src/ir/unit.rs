// Copyright (c) 2017-2020 Fabian Schuiki

// #![deny(missing_docs)]

use crate::{
    ir::{
        prelude::*, BlockData, ControlFlowGraph, DataFlowGraph, ExtUnit, ExtUnitData,
        FunctionInsertPos, FunctionLayout, InstBuilder, InstData, UnitId, ValueData,
    },
    table::TableKey,
    verifier::Verifier,
    Type,
};
use std::{
    collections::HashSet,
    ops::{Deref, Index, IndexMut},
};

/// A name of a function, process, or entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitName {
    /// An anonymous name, like `%42`.
    Anonymous(u32),
    /// A local name, like `%foo`.
    Local(String),
    /// A global name, like `@foo`.
    Global(String),
}

impl UnitName {
    // Create a new anonymous unit name.
    pub fn anonymous(id: u32) -> Self {
        UnitName::Anonymous(id)
    }

    // Create a new local unit name.
    pub fn local(name: impl Into<String>) -> Self {
        UnitName::Local(name.into())
    }

    // Create a new global unit name.
    pub fn global(name: impl Into<String>) -> Self {
        UnitName::Global(name.into())
    }

    /// Check whether this is a local name.
    ///
    /// Local names can only be linked within the same module.
    pub fn is_local(&self) -> bool {
        match self {
            UnitName::Anonymous(..) | UnitName::Local(..) => true,
            _ => false,
        }
    }

    /// Check whether this is a global name.
    ///
    /// Global names may be referenced by other modules and are considered by
    /// the global linker.
    pub fn is_global(&self) -> bool {
        match self {
            UnitName::Global(..) => true,
            _ => false,
        }
    }

    /// Get the underlying name.
    pub fn get_name(&self) -> Option<&str> {
        match self {
            UnitName::Global(n) | UnitName::Local(n) => Some(n.as_str()),
            _ => None,
        }
    }
}

impl std::fmt::Display for UnitName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UnitName::Anonymous(id) => write!(f, "%{}", id),
            UnitName::Local(n) => write!(f, "%{}", n),
            UnitName::Global(n) => write!(f, "@{}", n),
        }
    }
}

/// The three different units that may appear in LLHD IR.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    /// A `Function`.
    Function,
    /// A `Process`.
    Process,
    /// An `Entity`.
    Entity,
}

impl std::fmt::Display for UnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UnitKind::Function => write!(f, "func"),
            UnitKind::Process => write!(f, "proc"),
            UnitKind::Entity => write!(f, "entity"),
        }
    }
}

/// A function, process, or entity.
#[derive(Serialize, Deserialize)]
pub struct UnitData {
    pub kind: UnitKind,
    pub name: UnitName,
    pub sig: Signature,
    pub dfg: DataFlowGraph,
    pub cfg: ControlFlowGraph,
    pub layout: FunctionLayout,
}

impl UnitData {
    /// Create a new unit.
    pub fn new(kind: UnitKind, name: UnitName, sig: Signature) -> Self {
        match kind {
            UnitKind::Function => {
                assert!(!sig.has_outputs());
                assert!(sig.has_return_type());
            }
            UnitKind::Process | UnitKind::Entity => {
                assert!(!sig.has_return_type());
            }
        }
        let mut data = Self {
            kind,
            name,
            sig,
            dfg: DataFlowGraph::new(),
            cfg: ControlFlowGraph::new(),
            layout: FunctionLayout::new(),
        };
        if kind == UnitKind::Entity {
            let bb = data.cfg.add_block();
            data.layout.append_block(bb);
        }
        data.dfg.make_args_for_signature(&data.sig);
        data
    }
}

/// An immutable function, process, or entity.
#[derive(Clone, Copy)]
pub struct Unit<'a> {
    unit: UnitId,
    data: &'a UnitData,
}

impl<'a> Unit<'a> {
    /// Create a new unit wrapper around raw unit data.
    pub fn new(unit: UnitId, data: &'a UnitData) -> Self {
        Self { unit, data }
    }

    /// Create a new unit wrapper around raw unit data that has not been added
    /// to a module yet.
    pub fn new_anonymous(data: &'a UnitData) -> Self {
        Self::new(UnitId::invalid(), data)
    }

    /// Get the unit's id.
    #[inline(always)]
    pub fn id(self) -> UnitId {
        self.unit
    }

    /// Get the unit's data.
    #[inline(always)]
    pub fn data(self) -> &'a UnitData {
        self.data
    }

    /// Get the kind of this unit.
    pub fn kind(&self) -> UnitKind {
        self.data.kind
    }
}

/// Unfiltered.
impl<'a> Unit<'a> {
    /// Get the DFG of the unit being built.
    pub fn dfg(self) -> &'a DataFlowGraph {
        &self.data.dfg
    }

    /// Get the CFG of the unit being built.
    pub fn cfg(self) -> &'a ControlFlowGraph {
        &self.data.cfg
    }

    /// Get the CFG of the unit being built.
    pub fn try_cfg(self) -> Option<&'a ControlFlowGraph> {
        Some(&self.data.cfg)
    }

    /// Get the unit's layout.
    pub fn func_layout(self) -> &'a FunctionLayout {
        &self.data.layout
    }

    /// Get the unit's signature.
    pub fn sig(self) -> &'a Signature {
        &self.data.sig
    }

    /// Get the unit's name.
    pub fn name(self) -> &'a UnitName {
        &self.data.name
    }

    /// Dump the unit in human-readable form.
    #[deprecated(note = "simply drop the dump()")]
    pub fn dump(self) -> Self {
        self
    }

    /// Panic if the unit is not well-formed.
    pub fn verify(self) {
        let mut verifier = Verifier::new();
        verifier.verify_unit(self);
        match verifier.finish() {
            Ok(()) => (),
            Err(errs) => {
                eprintln!("");
                eprintln!("Verified {}:", self.data.kind);
                eprintln!("{}", self);
                eprintln!("");
                eprintln!("Verification errors:");
                eprintln!("{}", errs);
                panic!("verification failed");
            }
        }
    }

    /// Check if this unit is a `Function`.
    pub fn is_function(self) -> bool {
        self.kind() == UnitKind::Function
    }

    /// Check if this unit is a `Process`.
    pub fn is_process(self) -> bool {
        self.kind() == UnitKind::Process
    }

    /// Check if this unit is an `Entity`.
    pub fn is_entity(self) -> bool {
        self.kind() == UnitKind::Entity
    }

    /// Return an iterator over the unit's input arguments.
    pub fn input_args(self) -> impl Iterator<Item = Value> + 'a {
        self.sig().inputs().map(move |arg| self.arg_value(arg))
    }

    /// Return an iterator over the unit's output arguments.
    pub fn output_args(self) -> impl Iterator<Item = Value> + 'a {
        self.sig().outputs().map(move |arg| self.arg_value(arg))
    }

    /// Return an iterator over the unit's arguments.
    pub fn args(self) -> impl Iterator<Item = Value> + 'a {
        self.sig().args().map(move |arg| self.arg_value(arg))
    }

    /// Get the input argument at position `pos`.
    pub fn input_arg(self, pos: usize) -> Value {
        self.arg_value(
            self.sig()
                .inputs()
                .nth(pos)
                .expect("input argument position out of bounds"),
        )
    }

    /// Get the output argument at position `pos`.
    pub fn output_arg(self, pos: usize) -> Value {
        self.arg_value(
            self.sig()
                .outputs()
                .nth(pos)
                .expect("output argument position out of bounds"),
        )
    }

    /// Return the name of an external unit.
    pub fn extern_name(self, ext: ExtUnit) -> &'a UnitName {
        &self.dfg()[ext].name
    }

    /// Return the signature of an external unit.
    pub fn extern_sig(self, ext: ExtUnit) -> &'a Signature {
        &self.dfg()[ext].sig
    }

    // ----- Control Flow Graph ------------------------------------------------

    /// Return the name of a BB.
    pub fn get_block_name(self, bb: Block) -> Option<&'a str> {
        self.cfg().get_name(bb)
    }

    /// Return the anonymous name hint of a BB.
    pub fn get_anonymous_block_hint(self, bb: Block) -> Option<u32> {
        self.cfg().get_anonymous_hint(bb)
    }

    // ----- Data Flow Graph ---------------------------------------------------

    /// Check if a value is a placeholder.
    pub fn is_placeholder(self, value: Value) -> bool {
        self.dfg().is_placeholder(value)
    }

    /// Returns whether an instruction produces a result.
    pub fn has_result(self, inst: Inst) -> bool {
        self.dfg().has_result(inst)
    }

    /// Returns the result of an instruction.
    pub fn inst_result(self, inst: Inst) -> Value {
        self.dfg().inst_result(inst)
    }

    /// Returns the result of an instruction.
    pub fn get_inst_result(self, inst: Inst) -> Option<Value> {
        self.dfg().get_inst_result(inst)
    }

    /// Returns the value of an argument.
    pub fn arg_value(self, arg: Arg) -> Value {
        self.dfg().arg_value(arg)
    }

    /// Returns the type of a value.
    pub fn value_type(self, value: Value) -> Type {
        self.dfg().value_type(value)
    }

    /// Returns the type of an instruction.
    pub fn inst_type(self, inst: Inst) -> Type {
        self.dfg().inst_type(inst)
    }

    /// Return the argument that produces `value`.
    pub fn get_value_arg(self, value: Value) -> Option<Arg> {
        self.dfg().get_value_arg(value)
    }

    /// Return the argument that produces `value`, or panic.
    pub fn value_arg(self, value: Value) -> Arg {
        self.dfg().value_arg(value)
    }

    /// Return the instruction that produces `value`.
    pub fn get_value_inst(self, value: Value) -> Option<Inst> {
        self.dfg().get_value_inst(value)
    }

    /// Return the instruction that produces `value`, or panic.
    pub fn value_inst(self, value: Value) -> Inst {
        self.dfg().value_inst(value)
    }

    /// Return the name of a value.
    pub fn get_name(self, value: Value) -> Option<&'a str> {
        self.dfg().get_name(value)
    }

    /// Return the anonymous name hint of a value.
    pub fn get_anonymous_hint(self, value: Value) -> Option<u32> {
        self.dfg().get_anonymous_hint(value)
    }

    /// Iterate over all uses of a value.
    pub fn uses(self, value: Value) -> &'a HashSet<Inst> {
        self.dfg().uses(value)
    }

    /// Check if a value is used.
    pub fn has_uses(self, value: Value) -> bool {
        self.dfg().has_uses(value)
    }

    /// Check if a value has exactly one use.
    pub fn has_one_use(self, value: Value) -> bool {
        self.dfg().has_one_use(value)
    }

    /// Resolve a constant value.
    ///
    /// Returns `None` if the value is not constant. Note that this *does not*
    /// perform constant folding. Rather, the value must resolve to an
    /// instruction which produces a constant value.
    pub fn get_const(self, value: Value) -> Option<crate::Value> {
        self.dfg().get_const(value)
    }

    /// Resolve a constant time value.
    ///
    /// Returns `None` if the value is not constant. Note that this *does not*
    /// perform constant folding. Rather, the value must resolve to an
    /// instruction which produces a constant value.
    pub fn get_const_time(self, value: Value) -> Option<&'a crate::TimeValue> {
        self.dfg().get_const_time(value)
    }

    /// Resolve a constant integer value.
    ///
    /// Returns `None` if the value is not constant. Note that this *does not*
    /// perform constant folding. Rather, the value must resolve to an
    /// instruction which produces a constant value.
    pub fn get_const_int(self, value: Value) -> Option<&'a crate::IntValue> {
        self.dfg().get_const_int(value)
    }

    /// Resolve a constant array value.
    ///
    /// Returns `None` if the value is not constant. Note that this *does not*
    /// perform constant folding. Rather, the value must resolve to an
    /// instruction which produces a constant value.
    pub fn get_const_array(self, value: Value) -> Option<crate::ArrayValue> {
        self.dfg().get_const_array(value)
    }

    /// Resolve a constant struct value.
    ///
    /// Returns `None` if the value is not constant. Note that this *does not*
    /// perform constant folding. Rather, the value must resolve to an
    /// instruction which produces a constant value.
    pub fn get_const_struct(self, value: Value) -> Option<crate::StructValue> {
        self.dfg().get_const_struct(value)
    }

    /// Get the location hint associated with an instruction.
    ///
    /// Returns the byte offset of the instruction in the input file, or None if there
    /// is no hint for the instruction.
    pub fn location_hint(self, inst: Inst) -> Option<usize> {
        self.dfg().location_hint(inst)
    }
}

impl std::fmt::Display for Unit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {{\n",
            self.data.kind,
            self.data.name,
            self.data.sig.dump(self)
        )?;
        for bb in self.data.layout.blocks() {
            write!(f, "{}:\n", bb.dump(self))?;
            for inst in self.data.layout.insts(bb) {
                write!(f, "    {}\n", inst.dump(self))?;
            }
        }
        write!(f, "}}")?;
        Ok(())
    }
}

/// A mutable function, process, or entity.
pub struct UnitBuilder<'a> {
    /// The unit being modified.
    unit: Unit<'a>,
    /// The unit data being modified.
    data: &'a mut UnitData,
    /// The position where we are currently inserting instructions.
    pos: FunctionInsertPos,
}

// Ensure the UnitBuilder can be used like a Unit.
impl<'a> Deref for UnitBuilder<'a> {
    type Target = Unit<'a>;
    fn deref(&self) -> &Unit<'a> {
        &self.unit
    }
}

impl<'a> UnitBuilder<'a> {
    /// Create a new builder for a unit.
    pub fn new(unit: UnitId, data: &'a mut UnitData) -> Self {
        let pos = match data.kind {
            UnitKind::Entity => FunctionInsertPos::Append(data.layout.entry()),
            _ => FunctionInsertPos::None,
        };
        Self {
            unit: Unit::new(unit, unsafe { &*(data as *const _) }),
            // Safety of the above is enforced by UnitBuilder by requiring all
            // mutation of the unit to go through a mutable borrow of the
            // builder itself.
            data: data,
            pos,
        }
    }

    /// Create a new builder for a unit that has not yet been added to a module.
    pub fn new_anonymous(data: &'a mut UnitData) -> Self {
        Self::new(UnitId::invalid(), data)
    }

    /// Finish building and make the unit immutable again.
    pub fn finish(self) -> Unit<'a> {
        self.unit
    }

    /// Get the unit's mutable data.
    #[inline(always)]
    pub fn data(&mut self) -> &mut UnitData {
        self.data
    }
}

/// Unfiltered.
impl<'a> UnitBuilder<'a> {
    /// Get the unit's mutable signature.
    pub fn sig(&mut self) -> &mut Signature {
        &mut self.data.sig
    }

    /// Get the unit's mutable name.
    pub fn name(&mut self) -> &mut UnitName {
        &mut self.data.name
    }

    /// Return the unit being built.
    pub fn unit(&'a self) -> Unit<'a> {
        self.unit
    }

    /// Add a new instruction using an `InstBuilder`.
    pub fn ins(&mut self) -> InstBuilder<'a, '_> {
        InstBuilder::new(self)
    }

    /// Add a new instruction.
    pub fn build_inst(&mut self, data: InstData, ty: Type) -> Inst {
        let inst = self.data.dfg.add_inst(data, ty);
        self.pos.add_inst(inst, &mut self.data.layout);
        inst
    }

    /// Remove an instruction.
    pub fn remove_inst(&mut self, inst: Inst) {
        self.data.dfg.remove_inst(inst);
        self.pos.remove_inst(inst, &self.data.layout);
        self.data.layout.remove_inst(inst);
    }

    // Create a new BB.
    pub fn block(&mut self) -> Block {
        let bb = self.data.cfg.add_block();
        self.data.layout.append_block(bb);
        bb
    }

    /// Create a new named BB.
    pub fn named_block(&mut self, name: impl Into<String>) -> Block {
        let bb = self.block();
        self.data.cfg.set_name(bb, name.into());
        bb
    }

    /// Remove a BB.
    pub fn remove_block(&mut self, bb: Block) {
        let insts: Vec<_> = self.data.layout.insts(bb).collect();
        self.data.dfg.remove_block_use(bb);
        self.data.layout.remove_block(bb);
        self.data.cfg.remove_block(bb);
        for inst in insts {
            if self.data.dfg.has_result(inst) {
                let value = self.data.dfg.inst_result(inst);
                self.data.dfg.replace_use(value, Value::invalid());
            }
            self.data.dfg.remove_inst(inst);
        }
    }

    /// Append all following instructions at the end of the unit.
    pub fn insert_at_end(&mut self) {
        self.pos = FunctionInsertPos::Append(self.data.layout.entry());
    }

    /// Prepend all following instructions at the beginning of the unit.
    pub fn insert_at_beginning(&mut self) {
        self.pos = FunctionInsertPos::Prepend(self.data.layout.entry());
    }

    /// Append all following instructions to the end of `bb`.
    pub fn append_to(&mut self, bb: Block) {
        self.pos = FunctionInsertPos::Append(bb);
    }

    /// Prepend all following instructions to the beginning of `bb`.
    pub fn prepend_to(&mut self, bb: Block) {
        self.pos = FunctionInsertPos::Prepend(bb);
    }

    /// Insert all following instructions after `inst`.
    pub fn insert_after(&mut self, inst: Inst) {
        self.pos = FunctionInsertPos::After(inst);
    }

    /// Insert all following instructions before `inst`.
    pub fn insert_before(&mut self, inst: Inst) {
        self.pos = FunctionInsertPos::Before(inst);
    }

    /// Get the mutable DFG of the unit being built.
    pub fn dfg_mut(&mut self) -> &mut DataFlowGraph {
        &mut self.data().dfg
    }

    /// Get the mutable CFG of the unit being built.
    pub fn cfg_mut(&mut self) -> &mut ControlFlowGraph {
        &mut self.data().cfg
    }

    /// Get the mutable CFG of the unit being built.
    pub fn try_cfg_mut(&mut self) -> Option<&mut ControlFlowGraph> {
        Some(&mut self.data().cfg)
    }

    /// Get the function/process layout of the unit being built.
    pub fn func_layout_mut(&mut self) -> &mut FunctionLayout {
        &mut self.data().layout
    }

    /// Import an external unit for use within this unit.
    pub fn add_extern(&mut self, name: UnitName, sig: Signature) -> ExtUnit {
        self.dfg_mut().ext_units.add(ExtUnitData { sig, name })
    }

    /// Remove an instruction if its value is not being read.
    ///
    /// Returns true if the instruction was removed.
    pub fn prune_if_unused(&mut self, inst: Inst) -> bool {
        if self.dfg().has_result(inst) && !self.dfg().has_uses(self.dfg().inst_result(inst)) {
            #[allow(unreachable_patterns)]
            let inst_args: Vec<_> = self.dfg()[inst]
                .args()
                .iter()
                .cloned()
                .flat_map(|arg| self.dfg().get_value_inst(arg))
                .collect();
            self.remove_inst(inst);
            for inst in inst_args {
                self.prune_if_unused(inst);
            }
            true
        } else {
            false
        }
    }

    // ----- Control Flow Graph ------------------------------------------------

    /// Set the name of a BB.
    pub fn set_block_name(&mut self, bb: Block, name: String) {
        self.cfg_mut().set_name(bb, name)
    }

    /// Clear the name of a BB.
    pub fn clear_block_name(&mut self, bb: Block) -> Option<String> {
        self.cfg_mut().clear_name(bb)
    }

    /// Set the anonymous name hint of a BB.
    pub fn set_anonymous_block_hint(&mut self, bb: Block, hint: u32) {
        self.cfg_mut().set_anonymous_hint(bb, hint)
    }

    /// Clear the anonymous name hint of a BB.
    pub fn clear_anonymous_block_hint(&mut self, bb: Block) -> Option<u32> {
        self.cfg_mut().clear_anonymous_hint(bb)
    }

    // ----- Data Flow Graph ---------------------------------------------------

    /// Add a placeholder value.
    ///
    /// This function is intended to be used when constructing PHI nodes.
    pub fn add_placeholder(&mut self, ty: Type) -> Value {
        self.dfg_mut().add_placeholder(ty)
    }

    /// Remove a placeholder value.
    pub fn remove_placeholder(&mut self, value: Value) {
        self.dfg_mut().remove_placeholder(value)
    }

    /// Add an instruction.
    pub fn add_inst(&mut self, data: InstData, ty: Type) -> Inst {
        self.dfg_mut().add_inst(data, ty)
    }

    /// Set the name of a value.
    pub fn set_name(&mut self, value: Value, name: String) {
        self.dfg_mut().set_name(value, name)
    }

    /// Clear the name of a value.
    pub fn clear_name(&mut self, value: Value) -> Option<String> {
        self.dfg_mut().clear_name(value)
    }

    /// Set the anonymous name hint of a value.
    pub fn set_anonymous_hint(&mut self, value: Value, hint: u32) {
        self.dfg_mut().set_anonymous_hint(value, hint)
    }

    /// Clear the anonymous name hint of a value.
    pub fn clear_anonymous_hint(&mut self, value: Value) -> Option<u32> {
        self.dfg_mut().clear_anonymous_hint(value)
    }

    /// Replace all uses of a value with another.
    ///
    /// Returns how many uses were replaced.
    pub fn replace_use(&mut self, from: Value, to: Value) -> usize {
        self.dfg_mut().replace_use(from, to)
    }

    /// Replace the uses of a value with another, in a single instruction.
    ///
    /// Returns how many uses were replaced.
    pub fn replace_value_within_inst(&mut self, from: Value, to: Value, inst: Inst) -> usize {
        self.dfg_mut().replace_value_within_inst(from, to, inst)
    }

    /// Replace all uses of a block with another.
    ///
    /// Returns how many blocks were replaced.
    pub fn replace_block_use(&mut self, from: Block, to: Block) -> usize {
        self.dfg_mut().replace_block_use(from, to)
    }

    /// Replace all uses of a block with another, in a single instruction.
    ///
    /// Returns how many blocks were replaced.
    pub fn replace_block_within_inst(&mut self, from: Block, to: Block, inst: Inst) -> usize {
        self.dfg_mut().replace_block_within_inst(from, to, inst)
    }

    /// Remove all uses of a block.
    ///
    /// Replaces all uses of the block with an invalid block placeholder, and
    /// removes phi node entries for the block.
    ///
    /// Returns how many blocks were removed.
    pub fn remove_block_use(&mut self, block: Block) -> usize {
        self.dfg_mut().remove_block_use(block)
    }

    /// Remove all uses of a block, from a single instruction.
    ///
    /// Replaces all uses of the block with an invalid block placeholder, and
    /// removes phi node entries for the block.
    ///
    /// Returns how many blocks were removed.
    pub fn remove_block_from_inst(&mut self, block: Block, inst: Inst) -> usize {
        self.dfg_mut().remove_block_from_inst(block, inst)
    }

    /// Add a location hint to an instruction.
    ///
    /// Annotates the byte offset of an instruction in the input file.
    pub fn set_location_hint(&mut self, inst: Inst, loc: usize) {
        self.dfg_mut().set_location_hint(inst, loc)
    }
}

// Allow builders to be borrowed as the unit being built.

impl<'a> std::borrow::Borrow<Unit<'a>> for UnitBuilder<'a> {
    fn borrow(&self) -> &Unit<'a> {
        &self.unit
    }
}

// Allow immutable indexing into `Unit`.

impl Index<Value> for Unit<'_> {
    type Output = ValueData;
    fn index(&self, idx: Value) -> &ValueData {
        self.data.dfg.index(idx)
    }
}

impl Index<Inst> for Unit<'_> {
    type Output = InstData;
    fn index(&self, idx: Inst) -> &InstData {
        self.data.dfg.index(idx)
    }
}

impl Index<ExtUnit> for Unit<'_> {
    type Output = ExtUnitData;
    fn index(&self, idx: ExtUnit) -> &ExtUnitData {
        self.data.dfg.index(idx)
    }
}

impl Index<Block> for Unit<'_> {
    type Output = BlockData;
    fn index(&self, idx: Block) -> &BlockData {
        self.data.cfg.index(idx)
    }
}

// Allow immutable and mutable indexing into `UnitBuilder`.

impl Index<Value> for UnitBuilder<'_> {
    type Output = ValueData;
    fn index(&self, idx: Value) -> &ValueData {
        self.data.dfg.index(idx)
    }
}

impl Index<Inst> for UnitBuilder<'_> {
    type Output = InstData;
    fn index(&self, idx: Inst) -> &InstData {
        self.data.dfg.index(idx)
    }
}

impl Index<ExtUnit> for UnitBuilder<'_> {
    type Output = ExtUnitData;
    fn index(&self, idx: ExtUnit) -> &ExtUnitData {
        self.data.dfg.index(idx)
    }
}

impl Index<Block> for UnitBuilder<'_> {
    type Output = BlockData;
    fn index(&self, idx: Block) -> &BlockData {
        self.data.cfg.index(idx)
    }
}

impl IndexMut<Value> for UnitBuilder<'_> {
    fn index_mut(&mut self, idx: Value) -> &mut ValueData {
        self.data.dfg.index_mut(idx)
    }
}

impl IndexMut<Inst> for UnitBuilder<'_> {
    fn index_mut(&mut self, idx: Inst) -> &mut InstData {
        self.data.dfg.index_mut(idx)
    }
}

impl IndexMut<ExtUnit> for UnitBuilder<'_> {
    fn index_mut(&mut self, idx: ExtUnit) -> &mut ExtUnitData {
        self.data.dfg.index_mut(idx)
    }
}

impl IndexMut<Block> for UnitBuilder<'_> {
    fn index_mut(&mut self, idx: Block) -> &mut BlockData {
        self.data.cfg.index_mut(idx)
    }
}

#[allow(dead_code)]
mod static_checks {
    use super::*;

    pub fn ensure_send<'a>(u: Unit<'a>, ub: UnitBuilder<'a>) -> impl Send + 'a {
        (u, ub)
    }
}
