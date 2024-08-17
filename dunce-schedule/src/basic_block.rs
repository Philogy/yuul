use crate::ssa_block::{Block as SSABlock, Name, Statement, Value};
use ir::Literal;

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
    use crate::scheduler::MemoryScheduler;

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

        dbg!(bb.flatten_to().schedule_memory());
    }
}
