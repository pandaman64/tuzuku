use chumsky::Parser;

mod ast;
mod compiler;
mod opcode;
mod parser;
mod value;

fn main() {
    let source = r#"print("foobar")"#;
    println!("source = {}", source);
    match parser::parse().parse(source) {
        Ok(ast) => {
            let compiled = compiler::compile(ast);
            compiled.print();
        }
        Err(errors) => {
            for error in errors.iter() {
                eprintln!("error: {}", error)
            }
        }
    }
}
