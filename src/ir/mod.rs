// Copyright (c) 2017-2021 Fabian Schuiki

//! Representation of LLHD functions, processes, and entitites.
//!
//! This module implements the intermediate representation around which the rest
//! of the framework is built.
#![deny(missing_docs)]

use crate::{impl_table_key, ty::Type};

mod cfg;
mod dfg;
mod inst;
mod layout;
mod module;
pub mod prelude;
mod sig;
mod unit;

mod scope;
#[macro_use]
mod scoped_module;
mod module_tester;

use self::cfg::*;
use self::dfg::*;
pub use self::inst::*;
use self::layout::*;
pub use self::module::*;
pub use self::scope::*;
pub use self::sig::*;
pub use self::unit::*;

impl_table_key! {
    /// An instruction.
    struct Inst(u32) as "i";

    /// A value.
    struct Value(u32) as "v";

    /// A basic block.
    struct Block(u32) as "bb";

    /// An argument of a `Function`, `Process`, or `Entity`.
    struct Arg(u32) as "arg";

    /// An external `Function`, `Process` or `Entity`.
    struct ExtUnit(u32) as "ext";
}

impl Value {
    /// A placeholder for invalid values.
    ///
    /// This is used for unused instruction arguments.
    pub(crate) fn invalid() -> Self {
        Value(std::u32::MAX)
    }

    /// Check if this is a placeholder for invalid values.
    pub fn is_invalid(&self) -> bool {
        self.0 == std::u32::MAX
    }
}

impl Block {
    /// A placeholder for invalid blocks.
    ///
    /// This is used for unused instruction arguments.
    pub(crate) fn invalid() -> Self {
        Block(std::u32::MAX)
    }

    /// Check if this is a placeholder for invalid blocks.
    pub fn is_invalid(&self) -> bool {
        self.0 == std::u32::MAX
    }
}

/// Internal table storage for values.
#[allow(missing_docs)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ValueData {
    /// The invalid value placeholder.
    Invalid,
    /// The value is the result of an instruction.
    Inst { ty: Type, inst: Inst },
    /// The value is an argument of the `Function`, `Process`, or `Entity`.
    Arg { ty: Type, arg: Arg },
    /// The value is a placeholder. Used during PHI node construction.
    Placeholder { ty: Type },
}

impl ValueData {
    /// Check if the value is a placeholder.
    pub fn is_placeholder(&self) -> bool {
        match self {
            ValueData::Placeholder { .. } => true,
            _ => false,
        }
    }
}

impl Default for ValueData {
    fn default() -> ValueData {
        ValueData::Invalid
    }
}

/// Internal table storage for blocks.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BlockData {
    /// The name of the block.
    pub name: Option<String>,
}

/// Another unit referenced within a `Function`, `Process`, or `Entity`.
///
/// The linker will hook up external units to the actual counterparts as
/// appropriate.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtUnitData {
    /// The name of the referenced unit.
    pub name: UnitName,
    /// The signature of the referenced unit.
    pub sig: Signature,
}

impl Default for ExtUnitData {
    fn default() -> ExtUnitData {
        ExtUnitData {
            name: UnitName::Anonymous(0),
            sig: Signature::default(),
        }
    }
}

/// Any one of the table keys in this module.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnyObject {
    Inst(Inst),
    Value(Value),
    Block(Block),
    Arg(Arg),
}

impl From<Inst> for AnyObject {
    fn from(x: Inst) -> Self {
        AnyObject::Inst(x)
    }
}

impl From<Value> for AnyObject {
    fn from(x: Value) -> Self {
        AnyObject::Value(x)
    }
}

impl From<Block> for AnyObject {
    fn from(x: Block) -> Self {
        AnyObject::Block(x)
    }
}

impl From<Arg> for AnyObject {
    fn from(x: Arg) -> Self {
        AnyObject::Arg(x)
    }
}

impl std::fmt::Display for AnyObject {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AnyObject::Inst(x) => write!(f, "{}", x),
            AnyObject::Value(x) => write!(f, "{}", x),
            AnyObject::Block(x) => write!(f, "{}", x),
            AnyObject::Arg(x) => write!(f, "{}", x),
        }
    }
}

impl Value {
    /// Dump the value in human-readable form.
    pub fn dump<'a>(self, unit: &Unit<'a>) -> ValueDumper<'a> {
        ValueDumper(self, *unit)
    }
}

/// Temporary object to dump a `Value` in human-readable form for debugging.
pub struct ValueDumper<'a>(Value, Unit<'a>);

impl std::fmt::Display for ValueDumper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0.is_invalid() {
            write!(f, "%<invalid>")
        } else if let Some(name) = self.1.get_name(self.0) {
            write!(f, "%{}", name)
        } else if let Some(index) = self.1.get_anonymous_hint(self.0) {
            write!(f, "%{}", index)
        } else {
            write!(f, "%{}", self.0)
        }
    }
}

impl Block {
    /// Dump the basic block in human-readable form.
    pub fn dump<'a>(self, unit: &Unit<'a>) -> BlockDumper<'a> {
        BlockDumper(self, *unit)
    }
}

/// Temporary object to dump a `Block` in human-readable form for debugging.
pub struct BlockDumper<'a>(Block, Unit<'a>);

impl std::fmt::Display for BlockDumper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0.is_invalid() {
            write!(f, "<invalid>")
        } else if let Some(name) = self.1.get_block_name(self.0) {
            write!(f, "{}", name)
        } else if let Some(index) = self.1.get_anonymous_block_hint(self.0) {
            write!(f, "{}", index)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assembly;
    use crate::ir::scope::LLHDScope;

    use euclid::default::Box2D;

    use super::module_tester::LLHDModuleTester;

    scoped_llhd_module! {
        LLHDSlotMapWBoundingBox {
            LLHDKey,
            LLHDScope,
            bb: Box2D<usize>,
        }
    }

    #[test]
    fn default_llhd_slotmap_example() {
        let empty_llhd_slotmap = LLHDSlotMapWBoundingBox::default();
        let default_llhd_map = empty_llhd_slotmap.llhd_map;
        assert!(default_llhd_map.is_empty());
        let default_bb_map = empty_llhd_slotmap.bb;
        assert!(default_bb_map.is_empty());
    }

    #[test]
    fn from_llhd_module() {
        let module_txt = indoc::indoc! {"
            declare @bar (i32, i9) i32

            func @foo (i32 %x, i8 %y) i32 {
            %entry:
                %asdf0 = const i32 42
                %1 = const time 1.489ns 10d 9e
                %hello = alias i32 %asdf0
                %2 = not i32 %asdf0
                %3 = neg i32 %2
                %4 = add i32 %2, %3
                %5 = sub i32 %2, %3
                %6 = and i32 %2, %3
                %7 = or i32 %2, %3
                %8 = xor i32 %2, %3
                %cmp = eq i32 %7, %7
                br %cmp, %entry, %next
            %next:
                %a = exts i9, i32 %7, 4, 9
                %b = neg i9 %a
                %r = call i32 @bar (i32 %8, i9 %b)
                %many = [32 x i9 %b]
                %some = exts [9 x i9], [32 x i9] %many, 2, 9
                %one = extf i9, [9 x i9] %some, 3
                neg i9 %one
                ret i32 %3
            }

            entity @magic (i32$ %data, i1$ %clk) -> (i32$ %out) {
                %datap = prb i32$ %data
                %cmp = const i1 0
                reg i32$ %out, [%datap, rise %cmp]
            }
        "};
        let original_module = assembly::parse_module(module_txt).unwrap();
        let scoped_module: LLHDSlotMapWBoundingBox = original_module.clone().into();
        let converted_module: Module = scoped_module.into();
        let converted_module_tester = LLHDModuleTester::from(converted_module);
        let original_module_tester = LLHDModuleTester::from(original_module);
        assert_eq!(
            original_module_tester, converted_module_tester,
            "Module round-trip w/ Scoped Module failed."
        );
    }
}
