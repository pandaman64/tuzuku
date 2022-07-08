//! This module is solely for snapshot testing of the compiler/vm using insta.
#![cfg(test)]

use std::io::{self, Write};

use chumsky::prelude::Simple;

use crate::{parser::LineMapper, driver::Driver, opcode::Chunk};

fn assert_chunk_print(test_name: &str, chunk: &Chunk, _: &mut dyn Write) -> io::Result<()> {
    let mut chunk_print = vec![];
    let _ = chunk.write(&mut chunk_print);

    insta::assert_snapshot!(format!("{}_chunk_print", test_name), String::from_utf8_lossy(&chunk_print));

    Ok(())
}

fn assert_error_messages(test_name: &str, errors: Vec<Simple<char>>, _: &LineMapper) {
    let error_messages: Vec<String> = errors.iter().map(Simple::<char>::to_string).collect();

    insta::assert_yaml_snapshot!(format!("{}_error_messages", test_name), error_messages);
}

#[test]
fn test_print_string() {
    let test_name = "test_print_string";
    let source = r#"print("foobar");"#.to_string();
    let mut stdout = vec![];
    let mut driver = Driver {
        file_name: test_name.into(),
        source,
        run: true,
        stdout: &mut stdout,
        chunk_callback: assert_chunk_print,
        error_callback: assert_error_messages,
    };

    driver.run();

    insta::assert_snapshot!(format!("{}_stdout", test_name), String::from_utf8_lossy(&stdout));
}

#[test]
fn test_print_int() {
    let test_name = "test_print_int";
    let source = r#"print(42);"#.to_string();
    let mut stdout = vec![];
    let mut driver = Driver {
        file_name: test_name.into(),
        source,
        run: true,
        stdout: &mut stdout,
        chunk_callback: assert_chunk_print,
        error_callback: assert_error_messages,
    };

    driver.run();

    insta::assert_snapshot!(format!("{}_stdout", test_name), String::from_utf8_lossy(&stdout));
}
