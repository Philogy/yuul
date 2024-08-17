use std::collections::BTreeMap;

use crate::ssa_block::{Block as SSABlock, Name, Statement, Value};
use ir::{FunctionDefinition, Literal};

const RET_ADDR: &str = "__ret_addr__";
const COND: &str = "__cond__";

#[derive(Debug, Clone)]
pub enum Expr {
    Refr(String),
    Literal(Literal),
    Call { fn_name: String, args: Vec<Expr> },
}

impl Expr {
    pub fn r(name: &str) -> Self {
        Self::Refr(name.to_owned())
    }

    pub fn call1(name: &str, arg: Expr) -> Self {
        Self::call(name, vec![arg])
    }

    pub fn call2(name: &str, arg1: Expr, arg2: Expr) -> Self {
        Self::call(name, vec![arg1, arg2])
    }

    pub fn call(name: &str, args: Vec<Expr>) -> Self {
        Expr::Call {
            fn_name: name.to_owned(),
            args,
        }
    }
}

impl From<ir::Expr> for Expr {
    fn from(value: ir::Expr) -> Self {
        match value {
            ir::Expr::VarRef(vr) => Expr::Refr(vr),
            ir::Expr::Literal(literal) => Expr::Literal(literal),
            ir::Expr::Call { fn_name, args: ir_args } => {
                let mut args = Vec::new();
                for arg in ir_args {
                    args.push(arg.into());
                }
                Expr::Call { fn_name, args }
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
struct FlatStatementBuilder {
    next_intermed_id: usize,
    statements: Vec<Statement>,
}

impl FlatStatementBuilder {
    fn get_next_name(&mut self) -> Name {
        let id = self.next_intermed_id;
        self.next_intermed_id += 1;
        Name::Intermed(id)
    }

    fn flatten_to_values(&mut self, args: Vec<Expr>) -> Vec<Value> {
        args.into_iter()
            .rev()
            .map(|arg| match arg {
                Expr::Literal(x) => Value::Literal(x),
                Expr::Refr(name) => Value::RefName(name.into()),
                Expr::Call {
                    fn_name,
                    args: expr_args,
                } => {
                    let takes = self.flatten_to_values(expr_args);
                    let new_name = self.get_next_name();
                    self.statements.push(Statement::CallAssign {
                        assigns: vec![new_name.clone()],
                        calls: fn_name.clone(),
                        takes,
                    });
                    new_name.into()
                }
            })
            .rev()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Assignment {
    to_idents: Vec<String>,
    expr: Expr,
}

impl Assignment {
    fn new(to: Vec<&str>, expr: Expr) -> Self {
        Self {
            to_idents: to.into_iter().map(|s| s.to_owned()).collect(),
            expr,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    start_stack: Vec<String>,
    assignments: Vec<Assignment>,
    end_stack: Vec<String>,
}

struct BasicBlocksBuilder {
    start_stack: Vec<String>,
    current_stack: Vec<String>,
    assignments: Vec<Assignment>,
    functions: BTreeMap<String, Vec<BasicBlock>>,
    basic_blocks: Vec<BasicBlock>,
    loop_revert_state: Option<Vec<String>>,
    loop_continue_state: Option<Vec<String>>,
    fn_return: Option<Vec<String>>
}

impl BasicBlocksBuilder {
    fn new(start_stack: &Vec<String>) -> Self {
        Self {
            start_stack: start_stack.clone(),
            current_stack: start_stack.clone(),
            assignments: Vec::new(),
            functions: BTreeMap::new(),
            basic_blocks: Vec::new(),
            loop_revert_state: None,
            loop_continue_state: None,
            fn_return: None
        }
    }

    fn derive_builder(&self) -> Self {
        Self {
            start_stack: self.current_stack.clone(),
            current_stack: self.current_stack.clone(),
            assignments: Vec::new(),
            functions: BTreeMap::new(),
            basic_blocks: Vec::new(),
            loop_revert_state: self.loop_revert_state.clone(),
            loop_continue_state: self.loop_continue_state.clone(),
            fn_return: self.fn_return.clone()
        }
    }

    fn split_block(&mut self, block: ir::Block) {
        for statement in block.0 {
            match statement {
                ir::Statement::Block(block) => self.split_block(block),
                ir::Statement::FnDef(f) => self.split_fn_def(f),
                ir::Statement::Assignment { to, expr } => self.split_assignment(to, expr),
                ir::Statement::If { cond: _, body } => self.split_if(body),
                ir::Statement::Switch { cond, cases, default } => todo!(),
                ir::Statement::ForLoop { setup, cond, on_iter, body } => self.split_for(setup, cond, on_iter, body),
                ir::Statement::Leave => {
                    let bb = BasicBlock {
                        start_stack: self.start_stack.clone(),
                        assignments: self.assignments.clone(),
                        end_stack: self.fn_return.clone().unwrap(),
                    };
                    self.basic_blocks.push(bb);
                    self.assignments = Vec::new();
                    self.start_stack = self.current_stack.clone();
                },
                ir::Statement::Break => {
                    let mut bb = BasicBlock {
                        start_stack: self.start_stack.clone(),
                        assignments: self.assignments.clone(),
                        end_stack: self.current_stack.clone(),
                    };
                    if let Some(loop_revert_state) = self.loop_revert_state.clone() {
                        bb.end_stack = loop_revert_state
                    }
                    self.basic_blocks.push(bb);
                    self.assignments = Vec::new();
                    self.start_stack = self.current_stack.clone();
                },
                ir::Statement::Continue => {
                    let mut bb = BasicBlock {
                        start_stack: self.start_stack.clone(),
                        assignments: self.assignments.clone(),
                        end_stack: self.current_stack.clone(),
                    };
                    if let Some(loop_continue_state) = self.loop_continue_state.clone() {
                        bb.end_stack = loop_continue_state
                    }
                    self.basic_blocks.push(bb);
                    self.assignments = Vec::new();
                    self.start_stack = self.current_stack.clone();
                },
            }
        }
        if self.start_stack != self.current_stack || self.assignments.len() > 0 {
            let bb = BasicBlock {
                start_stack: self.start_stack.clone(),
                assignments: self.assignments.clone(),
                end_stack: self.current_stack.clone(),
            };
            self.basic_blocks.push(bb);
        }
        
    }

    fn split_for(&mut self, setup: ir::Block, cond: ir::Expr, on_iter: ir::Block, body: ir::Block) {
        let end_stack = self.current_stack.clone();
        let bb = BasicBlock {
            start_stack: self.start_stack.clone(),
            assignments: self.assignments.clone(),
            end_stack,
        };
        self.basic_blocks.push(bb);
        self.start_stack = self.current_stack.clone();
        self.assignments = Vec::new();
        
        let mut setup_builder = self.derive_builder();
        setup_builder.split_block(setup);
        
        let mut body_builder = setup_builder.derive_builder();
        body_builder.loop_revert_state = Some(setup_builder.start_stack.clone());
        body_builder.loop_continue_state = Some(setup_builder.current_stack.clone());
        body_builder.split_block(body);
        let mut last_bb = body_builder.basic_blocks.pop().unwrap();
        last_bb.end_stack = setup_builder.current_stack.clone();
        body_builder.basic_blocks.push(last_bb);

        let mut on_iter_builder = setup_builder.derive_builder();
        on_iter_builder.split_block(on_iter);
        let mut last_bb = on_iter_builder.basic_blocks.pop().unwrap();
        last_bb.end_stack = on_iter_builder.start_stack.clone();
        on_iter_builder.basic_blocks.push(last_bb);

        let mut cond_builder = setup_builder.derive_builder();
        let cond_var: String = COND.into();
        cond_builder.split_assignment(vec![cond_var.clone()], cond);
        let mut end_stack = cond_builder.start_stack.clone();
        end_stack.push(cond_var.clone());
        let bb = BasicBlock {
            start_stack: cond_builder.start_stack.clone(),
            assignments: cond_builder.assignments.clone(),
            end_stack,
        };
        cond_builder.basic_blocks.push(bb);
        let bb = BasicBlock {
            start_stack: cond_builder.start_stack.clone(),
            assignments: Vec::new(),
            end_stack: setup_builder.start_stack.clone(),
        };
        cond_builder.basic_blocks.push(bb);

        self.consume_builder(setup_builder);
        self.consume_builder(body_builder);
        self.consume_builder(on_iter_builder);
        self.consume_builder(cond_builder);
    }

    fn split_assignment(&mut self, to: Vec<String>, expr: ir::Expr) {
        for v in to.clone() {
            if !self.current_stack.contains(&v) {
                self.current_stack.push(v);
            }
        }
        let assignment = Assignment {
            to_idents: to,
            expr: expr.into(),
        };
        self.assignments.push(assignment);      
    }

    fn split_fn_def(&mut self, f: ir::FunctionDefinition) {
        let ret_addr: String = RET_ADDR.into();
        let FunctionDefinition { name, args, rets, body } = f;
        let mut start_stack = args;
        start_stack.extend(rets.clone());
        start_stack.push(ret_addr.clone());

        let mut assignments: Vec<Assignment> = Vec::new();
        for ret in &rets {
            assignments.push(Assignment {
                to_idents: vec![ret.to_owned()],
                expr: Expr::Literal([0u8;32]),
            });
        }

        let mut end_stack = rets;
        end_stack.push(ret_addr.clone());

        let mut builder = BasicBlocksBuilder::new(&start_stack);
        builder.fn_return = Some(end_stack.clone());
        builder.assignments = assignments;
        builder.split_block(body);
        let mut basic_blocks = builder.basic_blocks;
        let mut last_bb = basic_blocks.pop().unwrap();
        last_bb.end_stack = end_stack;
        basic_blocks.push(last_bb);
        self.functions.insert(name, basic_blocks);
        self.functions.extend(builder.functions);
    }

    fn split_if(&mut self, body: ir::Block) {
        let mut end_stack = self.current_stack.clone();
        end_stack.push(COND.into());
        let bb = BasicBlock {
            start_stack: self.start_stack.clone(),
            assignments: self.assignments.clone(),
            end_stack,
        };
        self.basic_blocks.push(bb);
        self.start_stack = self.current_stack.clone();
        self.assignments = Vec::new();
        let mut builder = self.derive_builder();
        builder.split_block(body);
        self.consume_builder(builder);
    }

    fn consume_builder(&mut self, builder: BasicBlocksBuilder) {
        self.basic_blocks.extend(builder.basic_blocks);
        self.functions.extend(builder.functions);
    }
}

impl BasicBlock {
    fn flatten_to(self) -> SSABlock {
        let mut flattener = FlatStatementBuilder::default();

        for assign in self.assignments {
            let new_stmt = match assign.expr {
                Expr::Literal(lit) => match assign.to_idents.as_slice() {
                    [] => None,
                    [ident] => Some(Statement::ValueAssign {
                        to: ident.into(),
                        value: Value::Literal(lit),
                    }),
                    _ => panic!("Assigning literal to more than one variable"),
                },
                Expr::Refr(value_ident) => match assign.to_idents.as_slice() {
                    [] => None,
                    [to_ident] => Some(Statement::ValueAssign {
                        to: to_ident.into(),
                        value: Value::RefName(value_ident.into()),
                    }),
                    _ => panic!("Assigning literal to more than one variable"),
                },
                Expr::Call { fn_name, args } => Some(Statement::CallAssign {
                    assigns: assign
                        .to_idents
                        .into_iter()
                        .map(|ident| ident.into())
                        .collect(),
                    calls: fn_name,
                    takes: flattener.flatten_to_values(args),
                }),
            };
            if let Some(stmt) = new_stmt {
                flattener.statements.push(stmt);
            }
        }

        SSABlock {
            start_stack: self.start_stack,
            statements: flattener.statements,
            end_stack: self.end_stack,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bb_assign() {
        let s1 = ir::Statement::Assignment {
            to: vec!["a".into(), "b".into(), "c".into()], 
            expr: ir::Expr::Literal([0u8; 32])
        };
        let s2 = ir::Statement::Assignment {
            to: vec!["a".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };
        let block = ir::Block { 0: vec![s1, s2] };
        let mut builder = BasicBlocksBuilder::new(&vec![]);
        builder.split_block(block);
        dbg!(builder.basic_blocks);
        dbg!(builder.functions);
    }

    #[test]
    fn test_bb_assign_fndef_assign() {
        let s1 = ir::Statement::Assignment {
            to: vec!["a".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };
        let f = ir::Statement::FnDef(
            ir::FunctionDefinition {
                name: "bla".into(),
                args: vec!["x".into(), "y".into()],
                rets: vec!["z".into()],
                body: ir::Block{ 0: vec![ir::Statement::Assignment {
                    to: vec!["a".into()],
                    expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
                }]},
            }
        );
        let s2 = ir::Statement::Assignment {
            to: vec!["b".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };
        let block = ir::Block { 0: vec![s1, f, s2] };
        let mut builder = BasicBlocksBuilder::new(&vec![]);
        builder.split_block(block);
        dbg!(builder.basic_blocks);
        dbg!(builder.functions);
    }

    #[test]
    fn test_bb_if() {
        let s1 = ir::Statement::Assignment {
            to: vec!["a".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };
        
        let a1 = ir::Statement::Assignment {
            to: vec!["x".into()],
            expr: ir::Expr::Call { fn_name: "x_raise".into(), args: vec![] }
        };
        let if_stmt = ir::Statement::If {
            cond: ir::Expr::Call { fn_name: "if_var".into(), args: vec![] },
            body: ir::Block { 0: vec![ir::Statement::Assignment {
                to: vec!["if".into()], 
                expr: ir::Expr::Call { fn_name: "nothing".into(), args: vec![] }
            }] }
        };
        let a2 = ir::Statement::Assignment {
            to: vec!["y".into()],
            expr: ir::Expr::Call { fn_name: "y_raise".into(), args: vec![] }
        };

        let s2 = ir::Statement::Assignment {
            to: vec!["b".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };

        let block = ir::Block{ 0: vec![s1, a1, if_stmt, a2, s2]};
        let mut builder = BasicBlocksBuilder::new(&vec![]);
        builder.split_block(block);
        dbg!(builder.basic_blocks);
        dbg!(builder.functions);
    }

    #[test]
    fn test_bb_for_loop_continue() {
        let s1 = ir::Statement::Assignment {
            to: vec!["a".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };
        let s2 = ir::Statement::Assignment {
            to: vec!["b".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };

        let setup = ir::Block {
            0: vec![ir::Statement::Assignment {
                to: vec!["i".into()],
                expr: ir::Expr::Literal([0u8; 32])
            }]
        };
        let cond = ir::Expr::Literal([1u8; 32]);
        let on_iter = ir::Block {
            0: vec![ir::Statement::Assignment {
                to: vec!["i".into()],
                expr: ir::Expr::Call { fn_name: "add".into(), args: vec![] }
            }]
        };
        let a1 = ir::Statement::Assignment {
            to: vec!["x".into()],
            expr: ir::Expr::Call { fn_name: "x_raise".into(), args: vec![] }
        };
        let if_stmt = ir::Statement::If {
            cond: ir::Expr::Call { fn_name: "continue".into(), args: vec![] },
            body: ir::Block { 0: vec![ir::Statement::Continue] }
        };
        let a2 = ir::Statement::Assignment {
            to: vec!["y".into()],
            expr: ir::Expr::Call { fn_name: "y_raise".into(), args: vec![] }
        };
        let body = ir::Block{ 0: vec![a1, if_stmt, a2]};
        let for_loop = ir::Statement::ForLoop {
            setup,
            cond,
            on_iter,
            body
        };
        let block = ir::Block { 0: vec![s1, s2, for_loop] };
        let mut builder = BasicBlocksBuilder::new(&vec![]);
        builder.split_block(block);
        dbg!(builder.basic_blocks);
        dbg!(builder.functions);
    }

    #[test]
    fn test_bb_for_loop_break() {
        let s1 = ir::Statement::Assignment {
            to: vec!["a".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };
        let s2 = ir::Statement::Assignment {
            to: vec!["b".into()],
            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![] }
        };

        let setup = ir::Block {
            0: vec![ir::Statement::Assignment {
                to: vec!["i".into()],
                expr: ir::Expr::Literal([0u8; 32])
            }]
        };
        let cond = ir::Expr::Literal([1u8; 32]);
        let on_iter = ir::Block {
            0: vec![ir::Statement::Assignment {
                to: vec!["i".into()],
                expr: ir::Expr::Call { fn_name: "add".into(), args: vec![] }
            }]
        };
        let a1 = ir::Statement::Assignment {
            to: vec!["x".into()],
            expr: ir::Expr::Call { fn_name: "x_raise".into(), args: vec![] }
        };
        let if_stmt = ir::Statement::If {
            cond: ir::Expr::Call { fn_name: "break".into(), args: vec![] },
            body: ir::Block { 0: vec![ir::Statement::Break] }
        };
        let a2 = ir::Statement::Assignment {
            to: vec!["y".into()],
            expr: ir::Expr::Call { fn_name: "y_raise".into(), args: vec![] }
        };
        let body = ir::Block{ 0: vec![a1, if_stmt, a2]};
        let for_loop = ir::Statement::ForLoop {
            setup,
            cond,
            on_iter,
            body
        };
        let block = ir::Block { 0: vec![s1, s2, for_loop] };
        let mut builder = BasicBlocksBuilder::new(&vec![]);
        builder.split_block(block);
        dbg!(builder.basic_blocks);
        dbg!(builder.functions);
    }

    #[test]
    fn test_bb_fndef() {
        let f = ir::Statement::FnDef(
            ir::FunctionDefinition {
                name: "bla".into(),
                args: vec!["x".into(), "y".into()],
                rets: vec!["z".into()],
                body: ir::Block{ 
                    0: vec![
                        ir::Statement::Assignment {
                            to: vec!["a".into()],
                            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![], }
                        },
                        ir::Statement::Assignment {
                            to: vec!["b".into()],
                            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![], }
                        },
                        ir::Statement::If {
                            cond: ir::Expr::Call { fn_name: "leave".into(), args: vec![] },
                            body: ir::Block { 0: vec![ir::Statement::Leave] }
                        },
                        ir::Statement::Assignment {
                            to: vec!["c".into()],
                            expr: ir::Expr::Call { fn_name: "bla".into(), args: vec![], }
                        },
                ]},
            }
        );
        let block = ir::Block { 0: vec![f] };
        let mut builder = BasicBlocksBuilder::new(&vec![]);
        builder.split_block(block);
        dbg!(builder.basic_blocks);
        dbg!(builder.functions);
    }

    #[test]
    fn test_flatten() {
        let bb = BasicBlock {
            start_stack: vec!["sender_slot".to_owned(), "amount".to_owned()],
            assignments: vec![Assignment::new(
                vec!["balance"],
                Expr::call2(
                    "add",
                    Expr::call1("sload", Expr::r("sender_slot")),
                    Expr::r("amount"),
                ),
            )],
            end_stack: vec!["amount".to_owned(), "sender_slot".to_owned()],
        };

        dbg!(bb.flatten_to());
    }
}
