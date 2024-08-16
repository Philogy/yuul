pub struct YulObject {
    pub code: Block,
    pub objects: Vec<(String, YulObject)>,
    pub data: Vec<(String, Vec<u8>)>,
}

pub struct Block(pub Vec<Statement>);

type Literal = [u8; 32];

pub enum Expr {
    VarRef(String),
    Literal(Literal),
    Call { fn_name: String, args: Vec<Expr> },
}

pub enum Statement {
    Block(Block),
    FnDef(FunctionDefinition),
    Assignment {
        to: Vec<String>,
        expr: Expr,
    },
    If {
        cond: Expr,
        body: Block,
    },
    Switch {
        cond: Expr,
        cases: Vec<(Literal, Block)>,
        default: Option<Block>,
    },
    ForLoop {
        setup: Block,
        cond: Expr,
        on_iter: Block,
        body: Block,
    },
    Leave,
    Break,
    Continue,
}

pub struct FunctionDefinition {
    pub name: String,
    pub args: Vec<String>,
    pub rets: Vec<String>,
    pub body: Block,
}
