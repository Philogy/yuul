use crate::ssa_block::{Block, Name, Statement, Value};
use ir::Literal;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Op<'a> {
    Swap(usize),
    Dup(usize),
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

fn inc_value_count(use_counts: &mut HashMap<Name, usize>, value: &Value) {
    match value {
        Value::RefName(name) => *use_counts.entry(name.clone()).or_default() += 1,
        Value::Literal(_) => (),
    }
}

struct MemoryAsRegisters {
    slots: Vec<Option<Name>>,
    counts: HashMap<Name, usize>,
}

impl MemoryAsRegisters {
    fn new(slots: Vec<Option<Name>>, counts: HashMap<Name, usize>) -> Self {
        Self { slots, counts }
    }

    fn len(&self) -> usize {
        self.slots.len()
    }

    fn get_count(&self, name: &Name) -> &usize {
        self.counts.get(name).unwrap_or(&0)
    }

    fn use_reference(&mut self, name: &Name) -> usize {
        let loc = self
            .get_loc(name)
            .unwrap_or_else(|| panic!("Undefined reference {:?}", name));
        let count = self.counts.entry(name.clone()).or_default();
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
        let slot = match slot {
            Some(i) => Some(i),
            None => self
                .slots
                .iter()
                .enumerate()
                .find(|(_, stored_name)| stored_name.is_none()),
        };
        slot.map(|(i, _)| i)
    }

    fn get_or_assign_loc(&mut self, name: &Name) -> usize {
        let slot = self.get_loc(name);
        slot.unwrap_or_else(|| {
            let index = self.len();
            self.slots.push(Some(name.clone()));
            index
        })
    }
}

impl MemoryScheduler for Block {
    fn schedule_memory(&self) -> (usize, Vec<Op>) {
        let mut memory: MemoryAsRegisters = {
            let mut start_counts: HashMap<Name, usize> = HashMap::new();

            for stmt in self.statements.iter() {
                match stmt {
                    Statement::ValueAssign { to: _, value } => {
                        inc_value_count(&mut start_counts, value)
                    }
                    Statement::CallAssign {
                        assigns: _,
                        calls: _,
                        takes,
                    } => takes
                        .iter()
                        .for_each(|value| inc_value_count(&mut start_counts, value)),
                }
            }

            for out_ref in self.end_stack.iter() {
                inc_value_count(&mut start_counts, &Value::RefName(out_ref.into()));
            }
            let start_slots = self
                .start_stack
                .iter()
                .map(|name| match start_counts.get(&name.into()).unwrap_or(&0) {
                    0 => None,
                    _ => Some(name.into()),
                })
                .collect();

            MemoryAsRegisters::new(start_slots, start_counts)
        };

        let mut ops: Vec<Op> = vec![];

        for stmt in self.statements.iter() {
            match stmt {
                Statement::ValueAssign { to, value } => match value {
                    Value::Literal(lit) => {
                        if *memory.get_count(to) > 0 {
                            ops.extend([
                                Op::Push(*lit),
                                Op::MemVarStore(memory.get_or_assign_loc(to)),
                            ]);
                        }
                    }
                    Value::RefName(name) => {
                        let from_loc = memory.use_reference(name);
                        ops.push(Op::MemCopy {
                            from: from_loc,
                            to: memory.get_or_assign_loc(to),
                        });
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
                    assigns
                        .iter()
                        .for_each(|name| ops.push(Op::MemVarStore(memory.get_or_assign_loc(name))));
                }
            }
        }

        for (i, name) in self.end_stack.iter().enumerate() {
            let from_loc = memory.use_reference(&name.into());
            if i != from_loc {
                memory.slots.swap(from_loc, i);
                ops.push(Op::MemSwap(i, from_loc));
            }
        }

        (memory.len(), ops)
    }
}
