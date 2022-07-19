//! This module is solely for snapshot testing of the compiler/vm using insta.
#![cfg(test)]

use std::io::{self, Write};

use chumsky::prelude::Simple;

use crate::{driver::Driver, opcode::Chunk, parser::LineMapper};

fn assert_chunk_print(test_name: &str, chunk: &Chunk, _: &mut dyn Write) -> io::Result<()> {
    let mut chunk_print = vec![];
    let _ = chunk.write(test_name, &mut chunk_print);

    insta::assert_snapshot!(
        format!("{}_chunk_print", test_name),
        String::from_utf8_lossy(&chunk_print)
    );

    Ok(())
}

fn assert_error_messages(test_name: &str, errors: Vec<Simple<char>>, _: &LineMapper) {
    let error_messages: Vec<String> = errors.iter().map(Simple::<char>::to_string).collect();

    insta::assert_yaml_snapshot!(format!("{}_error_messages", test_name), error_messages);
}

fn run_test(test_name: &str, source: &str) {
    let mut stdout = vec![];
    let mut driver = Driver {
        file_name: test_name.into(),
        source: source.into(),
        run: true,
        stdout: &mut stdout,
        chunk_callback: assert_chunk_print,
        error_callback: assert_error_messages,
    };

    driver.run();

    insta::assert_snapshot!(
        format!("{}_stdout", test_name),
        String::from_utf8_lossy(&stdout)
    );
}

#[test]
fn test_print_string() {
    run_test("test_print_string", r#"print("foobar");"#);
}

#[test]
fn test_print_int() {
    run_test("test_print_int", r#"print(42);"#);
}

#[test]
fn test_function_call() {
    run_test(
        "test_function_call",
        r#"
fun greet(name) {
    print("Hello");
    print(name);
}

greet("John Doe");
"#,
    )
}

#[test]
fn test_local_variable() {
    run_test(
        "test_local_variable",
        r#"
fun foo() {
    var variable = 100;
    print(variable);
    variable = "foo";
    print(variable);
}

fun bar() {
    var uninitialized;
    print(uninitialized);
    uninitialized = "initialized";
    print(uninitialized);
}

foo();
bar();
"#,
    );
}

#[test]
fn test_capture_no_escape() {
    run_test(
        "test_capture_no_escape",
        r#"
fun foo() {
    var foo1 = 100;
    var foo2 = 200;
    fun bar() {
        var bar1 = 300;
        print(foo2);
        print(bar1);
    }
    bar();
}

foo();
"#,
    );
}
