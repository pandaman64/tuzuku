use chumsky::Parser;

mod ast;
mod parser;

fn main() {
    let source = r#"print("foobar")"#;
    println!("source = {}", source);
    match parser::parse().parse(source) {
        Ok(ast) => match ast {
            ast::Ast::Print(content) => println!("printing: {}", content),
        },
        Err(errors) => {
            for error in errors.iter() {
                eprintln!("error: {}", error)
            }
        }
    }
}
