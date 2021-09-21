use ovl_derive::FromParsedResource;
use std::fs::File;
use std::io::Read;

#[derive(Debug)]
pub enum ParsedObject {
    Resource(ParsedResource),
}

#[derive(Debug, Clone)]
pub struct ParsedResource {
    resource_name: String,
    resource_type: String,
    content: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Resource {
    resource_name: String,
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

// For later
//pub enum AstNode {
//    Resource(ParsedResource),
//    Integer(i32),
//    DoublePrecisionFloat(f64),
//}

peg::parser! {
  pub grammar parser() for str {

    pub rule ovl() -> Vec<ParsedObject> = _ o:resource()* _ {
        o
    }
    rule _() = [' ' | '\t' | '\r' | '\n']*

    rule value() -> String = int() / string()

    rule member_separator() = _ "," _

    rule string() -> String = "\"" s:$(!"\"" [_])* "\"" { s.into_iter().collect() }

    rule key() -> String = k:$(['a'..='z'])* { k.into_iter().collect() }

    rule resource_type() -> String = t:$(['A'..='Z']+ ['a'..='z']*) { t.to_string() }

    rule number() = int()

    rule resource() -> ParsedObject = _ resource_type:resource_type() _ resource_name:string() _ "{" _ content:member() ** member_separator() _ "}" _ {
        ParsedObject::Resource(
            ParsedResource {
                resource_name,
                resource_type,
                content,
            }
        )
    }

    rule member() -> (String, String) = k:key() key_value_separator() _ v:value() {
        (k, v)
    }

    rule key_value_separator() = ":"

    rule int() -> String = n:$("-"?['0'..='9']+) { n.to_string() }
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
    let object = &parsed_objects[0];
    let resource = match object {
        ParsedObject::Resource(res) => res.clone().parse(),
    };
    dbg!(resource);
}
