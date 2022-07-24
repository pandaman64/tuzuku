#![feature(slice_ptr_get, slice_ptr_len)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::io;

use crate::{driver::Driver, side_effect::PrintAllHandler};

mod allocator;
mod ast;
mod compiler;
mod constant;
mod driver;
mod insta;
mod opcode;
mod parser;
mod side_effect;
mod value;
mod vm;

fn main() {
    let source = r#"
fun foo() {
    var local = 100;
    fun bar() {
        return local + 200;
    }
    local = 400;
    return bar;
}

var cls = foo();
print(cls());
"#
    .to_string();
    println!("source = {}", source);

    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let mut handler = PrintAllHandler {
        stdout: &mut stdout,
        stderr: &mut stderr,
    };
    let mut driver = Driver {
        file_name: "inline source".into(),
        source,
        run: true,
        handler: &mut handler,
    };
    driver.run();
}
