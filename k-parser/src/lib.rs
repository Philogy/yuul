use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(tag = "node")]
pub enum KInner {
    KApply(KApply),
    KToken(KToken),
}

const STANDARD_PADDING: &str = "  ";

impl KInner {
    pub fn get_kapply(&self) -> Result<&KApply, String> {
        match self {
            Self::KApply(kapply) => Ok(kapply),
            Self::KToken(_) => Err("Expected kapply".to_owned()),
        }
    }

    pub fn get_token(&self) -> Result<&KToken, String> {
        match self {
            Self::KToken(ktoken) => Ok(ktoken),
            Self::KApply(_) => Err("Expected token".to_owned()),
        }
    }

    pub fn to_fmt_string(&self, depth: usize) -> String {
        match self {
            Self::KApply(kapply) => kapply.to_fmt_string(depth),
            Self::KToken(ktoken) => ktoken.to_fmt_string(depth),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct KAst {
    pub version: u32,
    pub term: KInner,
    pub format: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct KToken {
    pub token: String,
    pub sort: KSort,
}

impl KToken {
    fn to_fmt_string(&self, depth: usize) -> String {
        format!(
            "{}KToken {} {:?}",
            STANDARD_PADDING.repeat(depth),
            self.sort.name,
            self.token
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct KSort {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct KApply {
    pub label: KLabel,
    pub args: Vec<KInner>,
}

impl KApply {
    pub fn to_fmt_string(&self, depth: usize) -> String {
        let arg_strs: String = self
            .args
            .iter()
            .map(|arg| arg.to_fmt_string(depth + 1))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{}KApply {}\n{}",
            STANDARD_PADDING.repeat(depth),
            self.label.name,
            arg_strs
        )
    }

    pub fn unpack_arg(&self) -> Result<&KInner, String> {
        match &self.args[..] {
            [a] => Ok(a),
            _ => Err("Expected 1 arg".to_owned()),
        }
    }

    pub fn unpack_2args(&self) -> Result<(&KInner, &KInner), String> {
        match &self.args[..] {
            [a, b] => Ok((a, b)),
            _ => Err("Expected 2 args".to_owned()),
        }
    }

    pub fn unpack_4args(&self) -> Result<(&KInner, &KInner, &KInner, &KInner), String> {
        match &self.args[..] {
            [a, b, c, d] => Ok((a, b, c, d)),
            _ => Err("Expected 4 args".to_owned()),
        }
    }

    pub fn flatten_cons(&self, cons_label: &str) -> Result<Vec<&KInner>, String> {
        if self.label.name != cons_label {
            return Ok(vec![]);
        }
        let mut values = vec![&self.args[0]];
        match &self.args[1] {
            KInner::KApply(kapply) => values.extend(kapply.flatten_cons(cons_label)?),
            KInner::KToken(_) => {
                return Err("Second element of cons list was token".into());
            }
        }
        Ok(values)
    }
}

impl fmt::Display for &KApply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_fmt_string(0))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct KLabel {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_deserialize_hex_data_literal() {
        let ast: KAst =
            serde_json::from_str(include_str!("../tests/hex-data-literal.yul.json")).unwrap();
        dbg!(&ast);
    }

    #[test]
    fn test_deserialize_block() {
        let ast: KAst = serde_json::from_str(include_str!("../tests/block.yul.json")).unwrap();
        dbg!(&ast);
    }
}
