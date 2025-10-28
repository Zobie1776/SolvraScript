//=============================================
// nova_core/src/backend/register_alloc.rs
//=============================================
// Author: NovaCore Team
// License: MIT
// Goal: Baseline register allocation infrastructure
// Objective: Provide common data structures and a linear scan allocator skeleton
//=============================================

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::backend::ir::{Function, IrType, ValueId};

//=============================================
// SECTION 1: Register Classes & Layout
//=============================================

/// Describes a physical register class (general, floating point, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterClass {
    General,
    Float,
}

/// Identifier for a physical register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalRegister {
    pub class: RegisterClass,
    pub index: u8,
}

impl PhysicalRegister {
    pub fn new(class: RegisterClass, index: u8) -> Self {
        Self { class, index }
    }
}

/// Result of the register allocation pass.
#[derive(Debug, Default, Clone)]
pub struct AllocationResult {
    value_map: BTreeMap<ValueId, AllocationSlot>,
}

impl AllocationResult {
    pub fn assign(&mut self, value: ValueId, slot: AllocationSlot) {
        self.value_map.insert(value, slot);
    }

    pub fn slot(&self, value: &ValueId) -> Option<&AllocationSlot> {
        self.value_map.get(value)
    }
}

/// Allocation slot either refers to a register or a spill stack slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationSlot {
    Register(PhysicalRegister),
    Spill(u32),
}

/// Trait implemented by backends to describe available registers.
pub trait RegisterLayout {
    /// Ordered list of general purpose registers.
    fn general_purpose(&self) -> &[PhysicalRegister];
    /// Ordered list of floating point registers.
    fn float_registers(&self) -> &[PhysicalRegister];
}

//=============================================
// SECTION 2: Linear Scan Allocator Skeleton
//=============================================

/// Baseline linear scan allocator.
pub struct LinearScanAllocator<'a> {
    layout: &'a dyn RegisterLayout,
    live_ranges: BTreeMap<ValueId, LiveInterval>,
    active: BTreeSet<ValueId>,
    next_spill: u32,
}

impl<'a> LinearScanAllocator<'a> {
    pub fn new(layout: &'a dyn RegisterLayout) -> Self {
        Self {
            layout,
            live_ranges: BTreeMap::new(),
            active: BTreeSet::new(),
            next_spill: 0,
        }
    }

    /// Allocate registers for the provided function, returning the mapping.
    pub fn allocate(&mut self, function: &Function) -> Result<AllocationResult> {
        self.build_live_intervals(function);
        let mut order: Vec<ValueId> = self.live_ranges.keys().cloned().collect();
        order.sort_by_key(|value| self.live_ranges[value].start);

        let mut result = AllocationResult::default();
        for value in order {
            let (start, is_float) = {
                let interval = &self.live_ranges[&value];
                (interval.start, interval.is_float)
            };
            self.expire_old(start);
            let slot = if let Some(reg) = self.try_allocate_register(is_float) {
                self.active.insert(value);
                AllocationSlot::Register(reg)
            } else {
                AllocationSlot::Spill(self.spill_value())
            };
            if let Some(interval) = self.live_ranges.get_mut(&value) {
                interval.allocation = Some(slot);
            }
            result.assign(value, slot);
        }
        Ok(result)
    }

    fn build_live_intervals(&mut self, function: &Function) {
        self.live_ranges.clear();
        for (position, instr) in function.instructions().iter().enumerate() {
            if let Some(result_ty) = &instr.ty {
                let value = ValueId::from(instr.id);
                let is_float = matches!(result_ty, IrType::F32 | IrType::F64);
                self.live_ranges
                    .entry(value)
                    .and_modify(|interval| interval.end = position as u32)
                    .or_insert_with(|| {
                        LiveInterval::new(position as u32, position as u32, is_float)
                    });
            }

            for operand in &instr.operands {
                self.live_ranges
                    .entry(*operand)
                    .and_modify(|interval| interval.end = position as u32)
                    .or_insert_with(|| LiveInterval::new(position as u32, position as u32, false));
            }
        }
    }

    fn expire_old(&mut self, position: u32) {
        self.active.retain(|value| {
            let interval = &self.live_ranges[value];
            interval.end >= position
        });
    }

    fn try_allocate_register(&self, is_float: bool) -> Option<PhysicalRegister> {
        let pool = if is_float {
            self.layout.float_registers()
        } else {
            self.layout.general_purpose()
        };

        for &reg in pool {
            if !self.register_is_active(reg) {
                return Some(reg);
            }
        }
        None
    }

    fn register_is_active(&self, register: PhysicalRegister) -> bool {
        self.active.iter().any(|value| {
            self.live_ranges
                .get(value)
                .and_then(|interval| interval.allocation)
                .map(|slot| matches!(slot, AllocationSlot::Register(reg) if reg == register))
                .unwrap_or(false)
        })
    }

    fn spill_value(&mut self) -> u32 {
        let slot = self.next_spill;
        self.next_spill += 1;
        slot
    }
}

//=============================================
// SECTION 3: Live Interval Representation
//=============================================

#[derive(Debug, Clone)]
struct LiveInterval {
    start: u32,
    end: u32,
    is_float: bool,
    allocation: Option<AllocationSlot>,
}

impl LiveInterval {
    fn new(start: u32, end: u32, is_float: bool) -> Self {
        Self {
            start,
            end,
            is_float,
            allocation: None,
        }
    }
}

//=============================================
// SECTION 4: Tests
//=============================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::ir::{
        ConstantValue, FunctionBuilder, FunctionSignature, IrType, Module, Opcode,
    };

    struct TestLayout;
    impl RegisterLayout for TestLayout {
        fn general_purpose(&self) -> &[PhysicalRegister] {
            static REGS: [PhysicalRegister; 2] = [
                PhysicalRegister {
                    class: RegisterClass::General,
                    index: 0,
                },
                PhysicalRegister {
                    class: RegisterClass::General,
                    index: 1,
                },
            ];
            &REGS
        }

        fn float_registers(&self) -> &[PhysicalRegister] {
            static REGS: [PhysicalRegister; 1] = [PhysicalRegister {
                class: RegisterClass::Float,
                index: 0,
            }];
            &REGS
        }
    }

    #[test]
    fn allocates_registers_for_straight_line_code() {
        let mut module = Module::new();
        let sig = FunctionSignature::new(vec![IrType::I64, IrType::I64], IrType::I64);
        let func_id = module.add_function("add", sig);
        let mut builder = FunctionBuilder::new(&mut module, func_id);
        let entry = builder.append_block(Some("entry".into()));
        builder.position_at_end(entry);
        let constants = [
            builder.make_constant(ConstantValue::I64(1), IrType::I64, None),
            builder.make_constant(ConstantValue::I64(2), IrType::I64, None),
        ];
        let sum = builder.emit_value(
            Opcode::Add,
            constants.to_vec(),
            IrType::I64,
            Some("sum".into()),
        );
        builder.emit_terminator(Opcode::Return, vec![sum], None);
        drop(builder);

        let function = module.function(func_id).clone();
        let layout = TestLayout;
        let mut allocator = LinearScanAllocator::new(&layout);
        let result = allocator.allocate(&function).expect("allocation succeeded");
        assert!(matches!(
            result.slot(&sum),
            Some(AllocationSlot::Register(_))
        ));
    }
}
