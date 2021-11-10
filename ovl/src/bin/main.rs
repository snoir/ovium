use ovl::parser;
use std::fs::File;
use std::io::Read;

pub fn main() {
    let mut ovlang_file = File::open("file1.ovl").unwrap();
    let mut ovlang_string = String::new();
    ovlang_file.read_to_string(&mut ovlang_string).unwrap();

    let parsed_objects = parser::ovl(&ovlang_string).unwrap();
    dbg!(parsed_objects);
}
