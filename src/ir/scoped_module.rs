//! Representation of linked LLHD units, with associated scope data.
//!
//! This module implements the `Module`, a collection of LLHD `Function`,
//! `Process`, and `Entity` objects linked together. A module acts as the root
//! node of an LLHD intermediate representation, and is the unit of information
//! ingested by the reader and emitted by the writer.

#[allow(unused_macros)]
macro_rules! scoped_llhd_module {
    // Match a struct definition with a list of secondary map types
    ($struct_name:ident { $key_type:ident, $main_value_type:ty, $( $sec_map_name:ident : $sec_value_type:ty ),* $(,)? }) => {

        use slotmap::{new_key_type, SecondaryMap};
        use crate::{
            index::*,
            ir::{
                DeclData, DeclId, ExtUnit, LinkedUnit, Signature, Unit, UnitBuilder, UnitData, UnitId,
                UnitName, Module, LLHDScope
            },
            table::{PrimaryTable, TableKey},
            verifier::Verifier,
        };
        use rayon::prelude::*;
        use std::collections::{BTreeSet, HashMap};
        use paste::paste;


        new_key_type! {
            pub struct $key_type;
        }

        /// A scoped module.
        ///
        /// This is the root node of an LLHD intermediate representation. Contains
        /// `Function`, `Process`, and `Entity` declarations and definitions.
        #[derive(Debug, Clone, Default)]
        pub struct $struct_name {
            pub(crate) units: PrimaryTable<UnitId, UnitData>,
            unit_order: BTreeSet<UnitId>,
            pub(crate) decls: PrimaryTable<DeclId, DeclData>,
            decl_order: BTreeSet<DeclId>,
            link_table: Option<HashMap<(UnitId, ExtUnit), LinkedUnit>>,
            location_hints: HashMap<UnitId, usize>,
            llhd_map: ScopedSlotMap<$key_type, $main_value_type, LLHDScope>,
            $(
                $sec_map_name: SecondaryMap<$key_type, $sec_value_type>,
            )*
        }

        impl $struct_name {
            /// Create a new empty module.
            pub fn new() -> Self {
                Self {
                    units: Default::default(),
                    unit_order: Default::default(),
                    decls: Default::default(),
                    decl_order: Default::default(),
                    link_table: Default::default(),
                    location_hints: Default::default(),
                    llhd_map: ScopedSlotMap::<$key_type, $main_value_type, LLHDScope>::default(),
                    $(
                        $sec_map_name: SecondaryMap::<$key_type, $sec_value_type>::default(),
                    )*
                }
            }

            // /// Dump the module in human-readable form.
            // pub fn dump(&self) -> ModuleDumper {
            //     ModuleDumper(self)
            // }

            /// Add a unit to the module.
            pub fn add_unit(&mut self, data: UnitData) -> UnitId {
                let unit = self.units.add(data);
                self.unit_order.insert(unit);
                self.link_table = None;
                unit
            }

            /// Remove a unit from the module.
            pub fn remove_unit(&mut self, unit: UnitId) {
                self.units.remove(unit);
                self.unit_order.remove(&unit);
            }

            /// Declare an external unit.
            pub fn declare(&mut self, name: UnitName, sig: Signature) -> DeclId {
                self.add_decl(DeclData {
                    name,
                    sig,
                    loc: None,
                })
            }

            /// Declare an external unit.
            pub fn add_decl(&mut self, data: DeclData) -> DeclId {
                let decl = self.decls.add(data);
                self.decl_order.insert(decl);
                self.link_table = None;
                decl
            }

            /// Remove a declaration from the module.
            pub fn remove_decl(&mut self, decl: DeclId) {
                self.decls.remove(decl);
                self.decl_order.remove(&decl);
            }

            /// Return an iterator over the units in this module.
            pub fn units(&self) -> impl Iterator<Item = Unit> {
                self.unit_order.iter().map(move |&id| self.unit(id))
            }

            /// Return a mutable iterator over the units in this module.
            pub fn units_mut(&mut self) -> impl Iterator<Item = UnitBuilder> {
                self.units
                    .storage
                    .iter_mut()
                    .map(|(&id, data)| UnitBuilder::new(UnitId::new(id), data))
            }

            /// Return a parallel iterator over the units in this module.
            pub fn par_units(&self) -> impl ParallelIterator<Item = Unit> {
                self.unit_order.par_iter().map(move |&id| self.unit(id))
            }

            /// Return a parallel mutable iterator over the units in this module.
            pub fn par_units_mut(&mut self) -> impl ParallelIterator<Item = UnitBuilder> {
                self.units
                    .storage
                    .par_iter_mut()
                    .map(|(&id, data)| UnitBuilder::new(UnitId::new(id), data))
            }

            /// Return an iterator over the functions in this module.
            pub fn functions(&self) -> impl Iterator<Item = Unit> {
                self.units().filter(|unit| unit.is_function())
            }

            /// Return an iterator over the processes in this module.
            pub fn processes(&self) -> impl Iterator<Item = Unit> {
                self.units().filter(|unit| unit.is_process())
            }

            /// Return an iterator over the entities in this module.
            pub fn entities(&self) -> impl Iterator<Item = Unit> {
                self.units().filter(|unit| unit.is_entity())
            }

            /// Return an iterator over the external unit declarations in this module.
            pub fn decls(&self) -> impl Iterator<Item = DeclId> + '_ {
                self.decl_order.iter().cloned()
            }

            /// Return an unit in the module.
            pub fn unit(&self, unit: UnitId) -> Unit {
                Unit::new(unit, &self[unit])
            }

            /// Return a mutable unit in the module.
            pub fn unit_mut(&mut self, unit: UnitId) -> UnitBuilder {
                self.link_table = None;
                UnitBuilder::new(unit, &mut self[unit])
            }

            /// Return an iterator over the symbols in the module.
            pub fn symbols(&self) -> impl Iterator<Item = (&UnitName, LinkedUnit, &Signature)> {
                self.units()
                    .map(|unit| (unit.name(), LinkedUnit::Def(unit.id()), unit.sig()))
                    .chain(
                        self.decls()
                            .map(move |decl| (&self[decl].name, LinkedUnit::Decl(decl), &self[decl].sig)),
                    )
            }

            /// Return an iterator over the local symbols in the module.
            pub fn local_symbols(
                &self,
            ) -> impl Iterator<Item = (&UnitName, LinkedUnit, &Signature)> {
                self.symbols().filter(|&(name, ..)| name.is_local())
            }

            /// Return an iterator over the global symbols in the module.
            pub fn global_symbols(
                &self,
            ) -> impl Iterator<Item = (&UnitName, LinkedUnit, &Signature)> {
                self.symbols().filter(|&(name, ..)| name.is_global())
            }

            /// Check whether the module is internally linked.
            ///
            /// Adding or modifying a unit invalidates the linkage within the module.
            pub fn is_linked(&self) -> bool {
                self.link_table.is_some()
            }

            /// Locally link the module.
            pub fn link(&mut self) {
                let mut failed = false;

                // Collect a table of symbols that we can resolve against.
                let mut symbols = HashMap::new();
                for (name, unit, sig) in self.symbols() {
                    if let Some((existing, _)) = symbols.insert(name, (unit, sig)) {
                        if !existing.is_decl() {
                            eprintln!("unit {} declared multiple times", name);
                            failed = true;
                        }
                    }
                }
                if failed {
                    panic!("linking failed; multiple uses of the same name");
                }

                // Resolve the external units in each unit.
                let mut linked = HashMap::new();
                for unit in self.units() {
                    for (ext_unit, data) in unit.extern_units() {
                        let (to, to_sig) = match symbols.get(&data.name).cloned() {
                            Some(to) => to,
                            None => {
                                eprintln!(
                                    "unit {} not found; referenced in {}",
                                    data.name,
                                    unit.name()
                                );
                                failed = true;
                                continue;
                            }
                        };
                        if to_sig != &data.sig {
                            eprintln!(
                                "signature mismatch: {} has {}, but reference in {} expects {}",
                                data.name,
                                to_sig,
                                unit.name(),
                                data.sig
                            );
                            failed = true;
                            continue;
                        }
                        linked.insert((unit.id(), ext_unit), to);
                    }
                }
                if failed {
                    panic!("linking failed; unresolved references");
                }
                self.link_table = Some(linked);
            }

            /// Panic if the module is not well-formed.
            pub fn verify(&self) {
                let mut verifier = Verifier::new();
                verifier.verify_module(self.units());
                match verifier.finish() {
                    Ok(()) => (),
                    Err(errs) => {
                        eprintln!("");
                        eprintln!("Verified module:");
                        // eprintln!("{}", self.dump());
                        eprintln!("");
                        eprintln!("Verification errors:");
                        eprintln!("{}", errs);
                        panic!("verification failed");
                    }
                }
            }

            /// Lookup what an external unit links to.
            ///
            /// The module must be linked for this to work.
            pub fn lookup_ext_unit(&self, ext_unit: ExtUnit, within: UnitId) -> Option<LinkedUnit> {
                self.link_table
                    .as_ref()
                    .and_then(|lt| lt.get(&(within, ext_unit)))
                    .cloned()
            }

            /// Add a location hint to a unit.
            ///
            /// Annotates the byte offset of a unit in the input file.
            pub fn set_location_hint(&mut self, mod_unit: UnitId, loc: usize) {
                self.location_hints.insert(mod_unit, loc);
            }

            /// Get the location hint associated with a unit.
            ///
            /// Returns the byte offset of the unit in the input file, or None if there
            /// is no hint for the value.
            pub fn location_hint(&self, mod_unit: UnitId) -> Option<usize> {
                self.location_hints.get(&mod_unit).cloned()
            }

            pub fn get_llhd_map(&self, key: $key_type) -> Option<&$main_value_type> {
                if let Some(data) = self.llhd_map.get(key) {
                    Some(data)
                } else {
                    None
                }
            }

            pub fn insert_llhd_map(&mut self, scope: LLHDScope, data: $main_value_type) -> $key_type {
                self.llhd_map.insert(scope, data)
            }

            $(
                paste! {
                    pub fn [<get_ $sec_map_name>](&self, key: $key_type) -> Option<&$sec_value_type> {
                        if let Some(data) = self.$sec_map_name.get(key) {
                            Some(data)
                        } else {
                            None
                        }
                    }
                }
            )*

            $(
                paste! {
                    pub fn [<insert_ $sec_map_name>](&mut self, key: $key_type, data: $sec_value_type) -> Option<$sec_value_type> {
                        self.$sec_map_name.insert(key, data)
                    }
                }
            )*
        }

        impl std::ops::Index<UnitId> for $struct_name {
            type Output = UnitData;
            fn index(&self, idx: UnitId) -> &UnitData {
                &self.units[idx]
            }
        }

        impl std::ops::IndexMut<UnitId> for $struct_name {
            fn index_mut(&mut self, idx: UnitId) -> &mut UnitData {
                self.link_table = None;
                &mut self.units[idx]
            }
        }

        impl std::ops::Index<DeclId> for $struct_name {
            type Output = DeclData;
            fn index(&self, idx: DeclId) -> &DeclData {
                &self.decls[idx]
            }
        }

        impl std::ops::IndexMut<DeclId> for $struct_name {
            fn index_mut(&mut self, idx: DeclId) -> &mut DeclData {
                self.link_table = None;
                &mut self.decls[idx]
            }
        }

        impl From<Module> for $struct_name {
            fn from(module: Module) -> Self {
                Self {
                    units: module.units,
                    unit_order: module.unit_order,
                    decls: module.decls,
                    decl_order: module.decl_order,
                    link_table: module.link_table,
                    location_hints: module.location_hints,
                    llhd_map: Default::default(),
                    $( $sec_map_name: SecondaryMap::<$key_type, $sec_value_type>::default(), )*
                }
            }
        }

        impl From<$struct_name> for Module {
            fn from(scoped_module: $struct_name) -> Self {
                Self {
                    units: scoped_module.units,
                    unit_order: scoped_module.unit_order,
                    decls: scoped_module.decls,
                    decl_order: scoped_module.decl_order,
                    link_table: scoped_module.link_table,
                    location_hints: scoped_module.location_hints,
                }
            }
        }
    };
}

// /// Temporary object to dump a `Module` in human-readable form for debugging.
// pub struct ModuleDumper<'a>(&'a $struct_name);
//
// impl std::fmt::Display for ModuleDumper<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         let mut newline = false;
//         for unit in self.0.units() {
//             if newline {
//                 writeln!(f, "")?;
//                 writeln!(f, "")?;
//             }
//             newline = true;
//             write!(f, "{}: ", unit.id())?;
//             write!(f, "{}", unit)?;
//         }
//         if newline && !self.0.decls().count() > 0 {
//             writeln!(f, "")?;
//         }
//         for decl in self.0.decls() {
//             if newline {
//                 writeln!(f, "")?;
//             }
//             newline = true;
//             let data = &self.0[decl];
//             write!(f, "declare {} {}", data.name, data.sig)?;
//         }
//         Ok(())
//     }
// }
