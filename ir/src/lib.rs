use k_parser::{KApply, KAst, KInner, KToken};
use num_bigint::BigUint;
use std::error::Error;

#[derive(Clone, Debug)]
pub struct YulObject {
    pub name: String,
    pub code: Block,
    pub objects: Vec<YulObject>,
    pub data: Vec<(String, Vec<u8>)>,
}

#[derive(Clone, Debug)]
pub struct Block(pub Vec<Statement>);

pub type Literal = [u8; 32];

#[derive(Clone, Debug)]
pub enum Expr {
    VarRef(String),
    Literal(Literal),
    Call { fn_name: String, args: Vec<Expr> },
    Builtin { fn_name: String, input: String },
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct FunctionDefinition {
    pub name: String,
    pub args: Vec<String>,
    pub rets: Vec<String>,
    pub body: Block,
}

const OBJECT_LIST_CONS_LABEL: &str = "___YUL-OBJECTS_ObjectList_Object_ObjectList";
const YUL_OBJECT_LABEL: &str =
    "object_{___}_YUL-OBJECTS_Object_StringLiteral_Code_DataList_ObjectList";

pub fn bytes_to_literal(bytes: Vec<u8>) -> Result<Literal, String> {
    let mut lit: Literal = [0u8; 32];

    if bytes.len() > 32 {
        return Err(format!("bytes too long: {:?}", bytes));
    }

    for (i, b) in bytes.into_iter().enumerate() {
        lit[31 - i] = b;
    }

    Ok(lit)
}

impl TryFrom<&KApply> for Expr {
    type Error = String;
    fn try_from(value: &KApply) -> Result<Self, Self::Error> {
        match value.label.name.as_str() {
            "function_call" | "function_call_values" => {
                let (call_ident, expressions) = value.unpack_2args()?;
                let args = expressions.get_kapply()?.flatten_cons("expression_list")?;
                let fn_name = call_ident.get_token()?.token.clone();
                match fn_name.as_str() {
                    "datasize" | "dataoffset" | "setimmutable" | "getimmutable" => match args[..] {
                        [token] => Ok(Self::Builtin {
                            fn_name,
                            input: token.get_token()?.token.clone(),
                        }),
                        _ => Err(format!(
                            "Invoking builtin {} with not exactly 1 arg: {:?}",
                            fn_name, args
                        )),
                    },
                    _ => Ok(Self::Call {
                        fn_name,
                        args: args
                            .into_iter()
                            .map(|k_inner| k_inner.try_into())
                            .collect::<Result<Vec<Expr>, String>>()?,
                    }),
                }
            }
            unk_name => Err(format!("Unkown expression kapply with name {}", unk_name)),
        }
    }
}
impl TryFrom<&KToken> for Expr {
    type Error = String;
    fn try_from(ktoken: &KToken) -> Result<Self, Self::Error> {
        match ktoken.sort.name.as_str() {
            "HexLiteral" => hex::decode(ktoken.token.strip_prefix("0x").unwrap())
                .map_err(|hex_error| format!("from hex error: {:?}", hex_error))
                .and_then(bytes_to_literal)
                .map(Expr::Literal),
            "Int" => ktoken
                .token
                .parse::<BigUint>()
                .map_err(|parse_err| format!("Failed to parse big uint {:?}", parse_err))
                .and_then(|num| bytes_to_literal(num.to_bytes_be()))
                .map(Expr::Literal),
            "Identifier" => Ok(Expr::VarRef(ktoken.token.clone())),
            _ => Err(format!("Unknown token {:?}", ktoken)),
        }
    }
}

impl TryFrom<&KInner> for Expr {
    type Error = String;
    fn try_from(value: &KInner) -> Result<Self, Self::Error> {
        match value {
            KInner::KApply(kapply) => kapply.try_into(),
            KInner::KToken(ktoken) => ktoken.try_into(),
        }
    }
}

impl TryFrom<&KInner> for Statement {
    type Error = String;
    fn try_from(k_inner: &KInner) -> Result<Self, Self::Error> {
        let value = k_inner.get_kapply()?;

        match value.label.name.as_str() {
            "block" => Ok(Self::Block(value.try_into()?)),
            "let" => {
                let (id_list, expr) = value.unpack_2args()?;
                let identifiers = id_list.get_kapply()?.flatten_cons("typed_id_list")?;
                let to = identifiers
                    .into_iter()
                    .map(|k_ident| Ok(k_ident.get_token()?.token.clone()))
                    .collect::<Result<Vec<String>, String>>()?;
                Ok(Self::Assignment {
                    to,
                    expr: expr.try_into()?,
                })
            }
            "if" => {
                let (expr, block) = value.unpack_2args()?;
                Ok(Self::If {
                    cond: expr.try_into()?,
                    body: block.get_kapply()?.try_into()?,
                })
            }
            "function_call" | "function_call_values" => Ok(Self::Assignment {
                to: vec![],
                expr: k_inner.try_into()?,
            }),
            "switch_default" => {
                let (expr, case_list) = value.unpack_2args()?;
                let cases = case_list
                    .get_kapply()?
                    .flatten_cons("case_list")?
                    .into_iter()
                    .map(|case| {
                        let (literal, block) = case.get_kapply()?.unpack_2args()?;
                        let parsed_expr: Expr = literal.get_token()?.try_into()?;
                        let literal: Literal = if let Expr::Literal(lit) = parsed_expr {
                            Ok(lit)
                        } else {
                            Err(format!(
                                "Expected literal as case entry, got: {:?}",
                                parsed_expr
                            ))
                        }?;

                        Ok((literal, block.get_kapply()?.try_into()?))
                    })
                    .collect::<Result<Vec<(Literal, Block)>, String>>()?;

                Ok(Self::Switch {
                    cond: expr.try_into()?,
                    cases,
                    default: None,
                })
            }
            unk_name => {
                println!("unk_name: {}", unk_name);
                println!("{}", value.to_fmt_string(0));
                Err(format!(
                    "Unimplemented name for statement conversion {}",
                    unk_name
                ))
            }
        }
    }
}

impl TryFrom<&KApply> for Block {
    type Error = String;

    fn try_from(value: &KApply) -> Result<Self, Self::Error> {
        if value.label.name != "block" {
            return Err(format!(
                "Expecting label block instead got {:?}",
                value.label.name
            ));
        }
        let statements = value
            .unpack_arg()?
            .get_kapply()?
            .flatten_cons("statement_list")?;
        let statements = statements
            .into_iter()
            .map(|stmt| stmt.try_into())
            .collect::<Result<Vec<Statement>, String>>()?;

        Ok(Self(statements))
    }
}

impl TryFrom<&KInner> for YulObject {
    type Error = String;

    fn try_from(value: &KInner) -> Result<Self, Self::Error> {
        let value = value.get_kapply()?;
        if value.label.name != YUL_OBJECT_LABEL {
            return Err(format!(
                "Unexpected labal {:?} expected {:?}",
                value.label.name, YUL_OBJECT_LABEL
            ));
        }
        let (name, code, data_list, inner_objs) = value.unpack_4args()?;
        let (statements, functions_defs) = code.get_kapply()?.unpack_2args()?;
        let statements = statements.get_kapply()?.flatten_cons("statement_list")?;

        let code_block = Block(
            statements
                .into_iter()
                .map(|stmt| stmt.try_into())
                .collect::<Result<Vec<Statement>, _>>()?,
        );

        let name = name.get_token()?.token.clone();
        let objects = inner_objs
            .get_kapply()?
            .flatten_cons(OBJECT_LIST_CONS_LABEL)?
            .into_iter()
            .map(|k_inner| k_inner.try_into())
            .collect::<Result<Vec<YulObject>, String>>()?;

        let data_list = data_list.get_kapply()?.flatten_cons("data_list")?;
        let data = data_list
            .into_iter()
            .map(|data_section| {
                let (name, data) = data_section.get_kapply()?.unpack_2args()?;

                Ok((
                    name.get_token()?.token.clone(),
                    hex::decode(
                        data.get_token()?
                            .token
                            .strip_prefix("hex\"")
                            .unwrap()
                            .strip_suffix("\"")
                            .unwrap(),
                    )
                    .map_err(|from_hex_err| format!("From hex error: {:?}", from_hex_err))?,
                ))
            })
            .collect::<Result<Vec<(String, Vec<u8>)>, String>>()?;

        Ok(YulObject {
            name,
            code: code_block,
            objects,
            data,
        })
    }
}

impl TryFrom<KAst> for YulObject {
    type Error = String;
    fn try_from(value: KAst) -> Result<Self, Self::Error> {
        let top_level_obj_list = value
            .term
            .get_kapply()?
            .flatten_cons(OBJECT_LIST_CONS_LABEL)?;

        match top_level_obj_list[..] {
            [] => Err("No yul object".to_owned()),
            [main_obj] => Ok(main_obj),
            _ => Err("More than one top-level yul object".to_owned()),
        }?
        .try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_parse_simple() {
        let kast: KAst =
            serde_json::from_str(include_str!("../../k-parser/tests/simple.yul.json")).unwrap();
        let yul_obj: YulObject = kast.try_into().unwrap();

        dbg!(&yul_obj);
    }
}
