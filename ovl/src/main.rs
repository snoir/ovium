use ovl_derive::FromParsedResource;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct ParsedResource {
    name: String,
    resource_type: String,
    content: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Resource {
    name: String,
    resource: ResourceType,
}

#[derive(Debug, Default, FromParsedResource)]
pub struct OvlFile {
    path: String,
    mode: i64,
    owner: String,
    group: String,
}

#[derive(Debug, Default, FromParsedResource)]
pub struct OvlCmd {
    command: String,
}

#[derive(Debug)]
pub enum ResourceType {
    File(OvlFile),
    Cmd(OvlCmd),
}

pub trait FromParsedResource {
    fn from_parsed_resource(parsed_resource: &ParsedResource) -> Resource;
}

impl ParsedResource {
    fn parse(self) -> Resource {
        match self.resource_type.as_str() {
            "File" => OvlFile::from_parsed_resource(&self),
            "Cmd" => OvlCmd::from_parsed_resource(&self),
            _ => panic!("Unknown resource type!"),
        }
    }
}

#[derive(Debug)]
pub enum Operator {
    Plus,
    Minus,
}

#[derive(Debug)]
pub enum RelOperator {
    Eq,
    Ge,
    Gt,
    Le,
    Lt,
    Ne,
}

// Idea: it might be nice / easier at some point to split Integer, Float,
// String and Array inside an other enum (Values ?).
#[derive(Debug)]
pub enum AstNode {
    Resource(Resource),
    Integer(i32),
    Float(f64),
    String(String),
    Array(Vec<AstNode>),
    IfStmt { cond: Box<Expr>, body: Vec<AstNode> },
    Variable { name: String, body: Box<AstNode> },
}

#[derive(Debug)]
pub enum Expr {
    Rel {
        op: RelOperator,
        lhs: Box<AstNode>,
        rhs: Box<AstNode>,
    },
}

peg::parser! {
    pub grammar parser() for str {
        pub rule ovl() -> Vec<AstNode> = _ o:node()* _ {
            o
        }

        rule _() = [' ' | '\t' | '\r' | '\n']*

        rule resource_value() -> String = float() / int() / string()

        rule member_separator() = _ "," _

        rule int() -> String = n:$("-"?['0'..='9']+) { n.to_string() }

        rule float() -> String = n:$("-"?['0'..='9']+"."['0'..='9']+) {
            n.to_string()
        }

        rule string() -> String = "\"" s:$(!"\"" [_])* "\"" {
            s.into_iter().collect()
        }

        rule unquoted_string() -> String = k:$(['a'..='z'])* {
            k.into_iter().collect()
        }

        rule node_string() -> AstNode = s:string() {
            AstNode::String(s)
        }

        rule node_int() -> AstNode = s:int() {
            AstNode::Integer(s.parse::<i32>().unwrap())
        }

        rule node_float() -> AstNode = s:float() {
            AstNode::Float(s.parse::<f64>().unwrap())
        }

        rule resource_type() -> String = t:$(['A'..='Z']+ ['a'..='z']*) {
            t.to_string()
        }

        rule resource() -> AstNode
            = _ resource_type:resource_type() _ name:string() _ "{"
              _ content:member() ** member_separator() _ "}" _ {
            AstNode::Resource(
                ParsedResource {
                    name,
                    resource_type,
                    content,
                }.parse()
            )
        }

        rule node() -> AstNode
            = resource() / if_stmt() / variable() / node_string() /
              node_float() / node_int() / array()

        rule member() -> (String, String)
            = k:unquoted_string() key_value_separator() _ v:resource_value() {
            (k, v)
         }

        rule key_value_separator() = ":"

        rule if_stmt() -> AstNode
            = "if" _ "(" _ rel_expr:expr() _ ")" _
              "{" _ body:(node())* _ "}" _ {
            AstNode::IfStmt { cond: Box::new(rel_expr), body }
        }

        rule expr() -> Expr
            = rel_expr_string() / rel_expr_float() / rel_expr_int()

        rule rel_expr_string() -> Expr
            = lhs:string() _ op:rel_operator() _ rhs:string() {
            Expr::Rel {
                op,
                lhs: Box::new(AstNode::String(lhs)),
                rhs: Box::new(AstNode::String(rhs)),
            }
        }

        rule rel_expr_int() -> Expr= lhs:int() _ op:rel_operator() _ rhs:int() {
            let lhs = lhs.parse::<i32>().unwrap();
            let rhs = rhs.parse::<i32>().unwrap();
            Expr::Rel {
                op,
                lhs: Box::new(AstNode::Integer(lhs)),
                rhs: Box::new(AstNode::Integer(rhs)),
            }
        }

        rule rel_expr_float() -> Expr
            = lhs:float() _ op:rel_operator() _ rhs:float() {
            let lhs = lhs.parse::<f64>().unwrap();
            let rhs = rhs.parse::<f64>().unwrap();
            Expr::Rel {
                op,
                lhs: Box::new(AstNode::Float(lhs)),
                rhs: Box::new(AstNode::Float(rhs)),
            }
        }

        rule operator() -> Operator = o:$("+" / "-") {
            match o {
                "+" => Operator::Plus,
                "-" => Operator::Minus,
                _ => unreachable!()
            }
        }

        rule rel_operator() -> RelOperator
            = r:$("==" / "<" / "<=" / ">" / ">=" / "!=") {
            match r {
                "==" => RelOperator::Eq,
                "<" => RelOperator::Lt,
                "<=" => RelOperator::Le,
                ">" => RelOperator::Gt,
                ">=" => RelOperator::Ge,
                "!=" => RelOperator::Ne,
                _ => unreachable!()
            }
        }

        rule variable() -> AstNode = _ n:unquoted_string() _ "=" _ v:node() _ {
            AstNode::Variable { name: n, body: Box::new(v) }
        }

        rule array() -> AstNode
            = "[" _ v:node() ** (_ "," _) _ "]" {
                for (i, member) in v.iter().enumerate() {
                    if (i > 0 && !(std::mem::discriminant(member)
                            == std::mem::discriminant(&v[i - 1]))) {
                        // To improve by returning Err
                        panic!("Array members are not of the same type!");
                    }
                }
                AstNode::Array(v)
        }
    }
}

fn value_from_key(keys: &[String], values: &[String], v: &str) -> String {
    let indice = keys.iter().position(|x| x == v).unwrap();
    values[indice].clone()
}

pub fn main() {
    let mut ovlang_file = File::open("file1.ovl").unwrap();
    let mut ovlang_string = String::new();
    ovlang_file.read_to_string(&mut ovlang_string).unwrap();

    let parsed_objects = parser::ovl(&ovlang_string).unwrap();
    dbg!(parsed_objects);
}
