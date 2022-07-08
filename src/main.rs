use std::io;

use chumsky::prelude::Simple;

use crate::{driver::Driver, opcode::Chunk, parser::LineMapper};

mod ast;
mod compiler;
mod driver;
mod opcode;
mod parser;
mod value;
mod vm;

fn print_errors(errors: Vec<Simple<char>>, mapper: &LineMapper) {
    for error in errors.iter() {
        eprintln!(
            "error at line {}: {}",
            mapper.find(error.span().start),
            error
        )
    }
}

fn main() {
    let source = r#"
fun greet(first, last) {
    print("hello ");
    print(first);
    print(" ");
    print(last);
}
greet("John", "Doe");

x = 100;
print(
    x
    - 2
    * 3
);
text = "foobar";
print(text);
"#
    .to_string();
    println!("source = {}", source);

    let mut stdout = io::stdout().lock();
    let mut driver = Driver {
        source,
        run: true,
        stdout: &mut stdout,
        chunk_callback: Chunk::print,
        error_callback: print_errors,
    };
    driver.run();
}
