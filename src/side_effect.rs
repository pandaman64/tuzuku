use std::{fmt::Display, io::{Write, self}};

use chumsky::prelude::Simple;

use crate::{value::Function, parser::LineMapper};

/// The side effect handlers performed by VM.
pub(crate) trait SideEffectHandler {
    fn compile_error(&mut self, file_name: &str, errors: Vec<Simple<char>>, mapper: &LineMapper) -> io::Result<()>;

    fn call_function(&mut self, function: &Function) -> io::Result<()>;

    fn print(&mut self, value: &dyn Display) -> io::Result<()>;
}

pub(crate) struct PrintAllHandler<'stdout, 'stderr> {
    pub(crate) stdout: &'stdout mut (dyn Write + 'stdout),
    pub(crate) stderr: &'stderr mut (dyn Write + 'stderr),
}

impl SideEffectHandler for PrintAllHandler<'_, '_> {
    fn compile_error(&mut self, _file_name: &str, errors: Vec<Simple<char>>, mapper: &LineMapper) -> io::Result<()> {
        for error in errors.iter() {
            writeln!(
                self.stderr,
                "error at line {}: {}",
                mapper.find(error.span().start),
                error
            )?;
        }

        Ok(())
    }

    fn call_function(&mut self, function: &Function) -> io::Result<()> {
        function.chunk().write(function.name(), self.stdout)
    }

    fn print(&mut self, value: &dyn Display) -> io::Result<()> {
        writeln!(self.stdout, "{}", value)
    }
}