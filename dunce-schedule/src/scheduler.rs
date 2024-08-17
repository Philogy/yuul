use crate::ssa_block::{Block, Name, Statement, Value};
use ir::Literal;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Op<'a> {
    Swap(usize),
    Dup(usize),
    Pop,
    Push(Literal),
    MemSwap(usize, usize),
    MemVarLoad(usize),
    MemVarStore(usize),
    MemCopy { from: usize, to: usize },
    CallFn(&'a str),
}

pub trait MemoryScheduler {
    fn schedule_memory(&self) -> (usize, Vec<Op>);
}

struct MemoryAsRegisters {
    slots: Vec<Option<Name>>,
    remaining_ref_counts: HashMap<Name, usize>,
}

impl MemoryAsRegisters {
    fn len(&self) -> usize {
        self.slots.len()
    }

    fn get_rem_ref_count(&self, name: &Name) -> &usize {
        self.remaining_ref_counts.get(name).unwrap_or(&0)
    }

    fn use_reference(&mut self, name: &Name) -> usize {
        let loc = self
            .get_loc(name)
            .unwrap_or_else(|| panic!("Undefined reference {:?}", name));
        let count = self.remaining_ref_counts.entry(name.clone()).or_default();
        *count = count
            .checked_sub(1)
            .unwrap_or_else(|| panic!("Referenced name {:?} with ref count 0", name));
        if *count == 0 {
            self.slots[loc] = None;
        }
        loc
    }

    fn get_loc(&self, name: &Name) -> Option<usize> {
        let slot = self
            .slots
            .iter()
            .enumerate()
            .find(|(_, stored_name)| match stored_name {
                Some(inner_name) => inner_name == name,
                None => false,
            });
        slot.map(|(i, _)| i)
    }

    fn get_or_assign_loc(&mut self, name: &Name) -> usize {
        let slot = self.get_loc(name);
        if let Some(slot) = slot {
            return slot;
        }

        match self
            .slots
            .iter()
            .enumerate()
            .find(|(_, stored_name)| stored_name.is_none())
        {
            Some((i, _)) => {
                self.slots[i] = Some(name.clone());
                i
            }
            None => {
                let index = self.len();
                self.slots.push(Some(name.clone()));
                index
            }
        }
    }
}
fn inc_value_count(use_counts: &mut HashMap<Name, usize>, value: &Value) {
    match value {
        Value::RefName(name) => *use_counts.entry(name.clone()).or_default() += 1,
        Value::Literal(_) => (),
    }
}

impl From<&Block> for MemoryAsRegisters {
    fn from(value: &Block) -> Self {
        let mut counts: HashMap<Name, usize> = HashMap::new();

        for stmt in value.statements.iter() {
            match stmt {
                Statement::ValueAssign { to: _, value } => inc_value_count(&mut counts, value),
                Statement::CallAssign {
                    assigns: _,
                    calls: _,
                    takes,
                } => takes
                    .iter()
                    .for_each(|value| inc_value_count(&mut counts, value)),
            }
        }

        for out_ref in value.end_stack.iter() {
            inc_value_count(&mut counts, &Value::RefName(out_ref.into()));
        }

        Self {
            remaining_ref_counts: counts,
            slots: vec![],
        }
    }
}

impl MemoryScheduler for Block {
    fn schedule_memory(&self) -> (usize, Vec<Op>) {
        let mut memory: MemoryAsRegisters = self.into();
        let mut ops: Vec<Op> = vec![];

        self.start_stack.iter().rev().for_each(|name| {
            let name = name.into();
            if *memory.get_rem_ref_count(&name) > 0 {
                let slot = memory.get_or_assign_loc(&name);
                ops.push(Op::MemVarStore(slot));
            } else {
                ops.push(Op::Pop);
            }
        });

        for stmt in self.statements.iter() {
            match stmt {
                Statement::ValueAssign { to, value } => match value {
                    Value::Literal(lit) => {
                        if *memory.get_rem_ref_count(to) > 0 {
                            ops.extend([
                                Op::Push(*lit),
                                Op::MemVarStore(memory.get_or_assign_loc(to)),
                            ]);
                        }
                    }
                    Value::RefName(name) => {
                        if *memory.get_rem_ref_count(&name) > 0 {
                            let from_loc = memory.use_reference(name);
                            ops.push(Op::MemCopy {
                                from: from_loc,
                                to: memory.get_or_assign_loc(to),
                            });
                        }
                    }
                },
                Statement::CallAssign {
                    assigns,
                    calls,
                    takes,
                } => {
                    takes.iter().rev().for_each(|value| match value {
                        Value::Literal(lit) => ops.push(Op::Push(*lit)),
                        Value::RefName(name) => {
                            ops.push(Op::MemVarLoad(memory.use_reference(name)))
                        }
                    });
                    ops.push(Op::CallFn(calls));
                    assigns.iter().rev().for_each(|name| {
                        if *memory.get_rem_ref_count(name) > 0 {
                            println!("name: {:?}", name);
                            dbg!(&memory.slots);
                            ops.push(Op::MemVarStore(memory.get_or_assign_loc(name)));
                            dbg!(&memory.slots);
                        } else {
                            ops.push(Op::Pop);
                        }
                    });
                }
            }
        }

        dbg!(&memory.slots);

        self.end_stack.iter().for_each(|name| {
            let loc = memory.use_reference(&name.into());
            ops.push(Op::MemVarLoad(loc));
        });

        (memory.len(), ops)
    }
}