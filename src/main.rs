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
"#;
    println!("source = {}", source);
    let parser = parser::parser(&arena);
    let mapper = LineMapper::new(source);
    match parser.parse(source) {
        Ok(ast) => {
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
                eprintln!(
                    "error at line {}: {}",
                    mapper.find(error.span().start),
                    error
                )
            }
        }
    }
}
