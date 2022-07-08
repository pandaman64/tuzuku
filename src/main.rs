use std::io;

use chumsky::Parser;

use crate::parser::LineMapper;

mod ast;
mod compiler;
mod opcode;
mod parser;
mod value;
mod vm;

fn main() {
    let arena = typed_arena::Arena::new();
    let source = r#"print(
    1
    - 2
    * 3
)"#;
    println!("source = {}", source);
    let parser = parser::parser(&arena);
    match parser.parse(source) {
        Ok(ast) => {
            let mapper = LineMapper::new(source);
            let compiled = compiler::compile(ast, &mapper);
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
