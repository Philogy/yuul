use ir::Literal;

#[derive(Clone, Debug)]
pub enum Name {
    Ident(String),
    Intermed(usize),
}

#[derive(Clone, Debug)]
pub enum Value {
    RefName(Name),
    Literal(Literal),
}

impl Value {
    pub fn ident(name: &String) -> Self {
        Self::RefName(name.into())
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self::Ident(value)
    }
}

impl From<&String> for Name {
    fn from(value: &String) -> Self {
        Self::Ident(value.clone())
    }
}

impl From<Name> for Value {
    fn from(value: Name) -> Self {
        Self::RefName(value)
    }
}

#[derive(Clone, Debug)]
pub enum Statement {
    CallAssign {
        assigns: Vec<Name>,
        calls: String,
        takes: Vec<Value>,
    },
    ValueAssign {
        to: Name,
        value: Value,
    },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub start_stack: Vec<String>,
    pub statements: Vec<Statement>,
    pub end_stack: Vec<String>,
}
