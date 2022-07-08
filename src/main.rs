use std::io;

use chumsky::Parser;

mod ast;
mod compiler;
mod opcode;
mod parser;
mod value;
mod vm;

fn main() {
    let source = r#"print("foobar")"#;
    println!("source = {}", source);
    match parser::parse().parse(source) {
        Ok(ast) => {
            let compiled = compiler::compile(ast);
            compiled.print();

            let mut stdout = io::stdout().lock();
            let mut vm = vm::Vm::new(compiled, &mut stdout);
            while !vm.done() {
                vm.step();
            }
        }
        Err(errors) => {
            for error in errors.iter() {
                eprintln!("error: {}", error)
            }
        }
    }
}
