#![feature(slice_ptr_get, slice_ptr_len)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::io::{self, Write};

use chumsky::prelude::Simple;

use crate::{driver::Driver, opcode::Chunk, parser::LineMapper};

mod allocator;
mod ast;
mod compiler;
mod constant;
mod driver;
mod insta;
mod opcode;
mod parser;
mod value;
mod vm;

fn print_chunk(file_name: &str, chunk: &Chunk, writer: &mut dyn Write) -> io::Result<()> {
    chunk.write(file_name, writer)
}

fn print_errors(_: &str, errors: Vec<Simple<char>>, mapper: &LineMapper) {
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
fun foo() {
    var foo = 1234;
    print("foo");
    return foo;
}

print(foo());
"#
    .to_string();
    println!("source = {}", source);

    let mut stdout = io::stdout().lock();
    let mut driver = Driver {
        file_name: "inline source".into(),
        source,
        run: true,
        stdout: &mut stdout,
        chunk_callback: print_chunk,
        error_callback: print_errors,
    };
    driver.run();
}
