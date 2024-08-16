#[derive(Debug, Clone)]
pub struct Expr {
    fn_name: String,
    args: Vec<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    assign_to: Vec<String>,
    expr: Expr,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    input_stack_names: Vec<String>,
    assignments: Vec<Assignment>,
    output_stack_names: Vec<String>,
}
