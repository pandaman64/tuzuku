//! This module is solely for snapshot testing of the compiler/vm using insta.
#![cfg(test)]

use std::io::{self, Write};

use chumsky::prelude::Simple;

use crate::{driver::Driver, parser::LineMapper, side_effect::SideEffectHandler};

struct InstaCapturingHandler {
    test_name: String,
    stdout: Vec<u8>,
}

impl SideEffectHandler for InstaCapturingHandler {
    fn compile_error(&mut self, file_name: &str, errors: Vec<Simple<char>>, _mapper: &LineMapper) -> io::Result<()> {
        let error_messages: Vec<String> = errors.iter().map(Simple::<char>::to_string).collect();
    
        insta::assert_yaml_snapshot!(format!("{}_{}_error_messages", self.test_name, file_name), error_messages);

        Ok(())
    }

    fn call_function(&mut self, function: &crate::value::Function) -> io::Result<()> {
        let mut chunk_print = vec![];
        let _ = function.chunk().write(function.name(), &mut chunk_print);
    
        insta::assert_snapshot!(
            format!("{}_{}_chunk_print", self.test_name, function.name()),
            String::from_utf8_lossy(&chunk_print)
        );
    
        Ok(())
    }

    fn print(&mut self, value: &dyn std::fmt::Display) -> io::Result<()> {
        writeln!(self.stdout, "{}", value)
    }
}

impl InstaCapturingHandler {
    fn new(test_name: &str) -> Self {
        Self { test_name: test_name.into(), stdout: vec![] }
    }
}

fn run_test(test_name: &str, source: &str) {
    let mut handler = InstaCapturingHandler::new(test_name);
    let mut driver = Driver {
        file_name: test_name.into(),
        source: source.into(),
        run: true,
        handler: &mut handler,
    };

    driver.run();

    insta::assert_snapshot!(
        format!("{}_stdout", test_name),
        String::from_utf8_lossy(&handler.stdout)
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

#[test]
fn test_capture_shared_local() {
    run_test(
        "test_capture_shared_local",
        r#"
fun main() {
    var slot;

    fun foo() {
        print(slot);
    }

    fun bar() {
        print(slot);
    }

    print(slot);
    slot = 1;
    foo();
    bar();

    slot = 2;
    foo();
    bar();
}

main();
"#,
    );
}

#[test]
fn test_return() {
    run_test(
        "test_return",
        r#"
fun foo() {
    var foo = 1234;
    print("foo");
    return foo;
}

print(foo());
"#,
    );
}
