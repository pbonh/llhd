use crate::ir::*;
use itertools::Itertools;
use std::collections::HashSet;

pub type LLHDArgs = Vec<(Value, Type)>;
pub type LLHDInstArgs = Vec<Opcode>;
pub type LLHDInstInfo = Vec<(Opcode, LLHDInstArgs)>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LLHDModuleTester {
    unit_args: HashSet<LLHDArgs>,
    unit_insts: HashSet<LLHDInstInfo>,
}

impl LLHDModuleTester {
    pub fn filter_nullary(unit: &Unit, inst_id: Inst) -> bool {
        let inst_data = &unit[inst_id];
        !matches!(inst_data, InstData::Nullary { .. })
    }
}

impl From<Module> for LLHDModuleTester {
    fn from(module: Module) -> Self {
        Self {
            unit_args: module
                .units()
                .map(|unit| {
                    unit.args()
                        .map(|arg| (arg, unit.value_type(arg)))
                        .collect_vec()
                })
                .collect(),
            unit_insts: module
                .units()
                .map(|unit| {
                    unit.all_insts()
                        .filter(|inst| Self::filter_nullary(&unit, *inst))
                        .map(|inst| {
                            (
                                unit[inst].opcode(),
                                unit[inst]
                                    .args()
                                    .iter()
                                    .filter(|inst_arg| !Value::is_invalid(inst_arg))
                                    .filter(|inst_arg| unit.get_value_inst(**inst_arg).is_some())
                                    .map(|inst_arg| unit[unit.value_inst(*inst_arg)].opcode())
                                    .collect_vec(),
                            )
                        })
                        .collect_vec()
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_module_single_entity() {
        let module_txt = indoc::indoc! {"
            entity @test_entity (i1 %in1, i1 %in2, i1 %in3) -> (i1$ %out1) {
                %null = const time 0s 1e
                %and1 = and i1 %in1, %in2
                %and2 = and i1 %in3, %in2
                %or1 = or i1 %and1, %and2
                drv i1$ %out1, %or1, %null
            }
        "};
        let module = crate::assembly::parse_module(module_txt).unwrap();
        let module_test_data = LLHDModuleTester::from(module);
        assert_eq!(
            module_test_data.unit_args.len(),
            1,
            "There is 1 Unit present in 2and_1or_common."
        );
        assert_eq!(
            module_test_data.unit_insts.len(),
            1,
            "There is 1 Unit present in 2and_1or_common."
        );
        let unit1_arg_data = module_test_data.unit_args.iter().next().unwrap();
        assert_eq!(unit1_arg_data.len(), 4, "There should be 4 Args in Unit.");
        let unit1_test_data = module_test_data.unit_insts.iter().next().unwrap();
        assert_eq!(unit1_test_data.len(), 5, "There should be 5 Insts in Unit.");

        let const_time_inst = &unit1_test_data[0];
        assert!(
            matches!(const_time_inst.0, Opcode::ConstTime),
            "Opcode for 1st Inst should be ConstTime."
        );

        let and1_inst = &unit1_test_data[1];
        let and1_inst_args = and1_inst.1.clone();
        assert!(
            matches!(and1_inst.0, Opcode::And),
            "Opcode for 2nd Inst should be And."
        );
        assert!(
            and1_inst_args.is_empty(),
            "And Inst Args are Unit Args, which have no type."
        );

        let and2_inst = &unit1_test_data[2];
        let and2_inst_args = and2_inst.1.clone();
        assert!(
            matches!(and2_inst.0, Opcode::And),
            "Opcode for 3rd Inst should be And."
        );
        assert!(
            and2_inst_args.is_empty(),
            "And Inst Args are Unit Args, which have no type."
        );

        let or1_inst = &unit1_test_data[3];
        let or1_inst_args = or1_inst.1.clone();
        assert!(
            matches!(or1_inst.0, Opcode::Or),
            "Opcode for 4th Inst should be Or."
        );
        assert!(
            matches!(or1_inst_args[0], Opcode::And),
            "Opcode for 1st Or Inst Arg should be And."
        );
        assert!(
            matches!(or1_inst_args[1], Opcode::And),
            "Opcode for 2nd Or Inst Arg should be And."
        );

        let drv_inst = &unit1_test_data[4];
        assert!(
            matches!(drv_inst.0, Opcode::Drv),
            "Opcode for 1st Inst should be Drv."
        );
    }

    #[test]
    fn from_module_mixed_units() {
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
        let module = crate::assembly::parse_module(module_txt).unwrap();
        let module_test_data = LLHDModuleTester::from(module);
        assert_eq!(
            module_test_data.unit_args.len(),
            2,
            "There is 2 Unit present in Module."
        );
    }
}
