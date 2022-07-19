use std::io;

use chumsky::{prelude::Simple, Parser};
use typed_arena::Arena;

use crate::{
    compiler,
    opcode::Chunk,
    parser::{self, LineMapper},
    vm::Vm,
};

pub(crate) struct Driver<'stdout> {
    pub(crate) file_name: String,
    pub(crate) source: String,
    pub(crate) run: bool,
    pub(crate) stdout: &'stdout mut (dyn io::Write + 'stdout),
    pub(crate) chunk_callback: fn(&str, &Chunk, &mut dyn io::Write) -> io::Result<()>,
    pub(crate) error_callback: fn(&str, Vec<Simple<char>>, &LineMapper),
}

impl<'stdout> Driver<'stdout> {
    pub(crate) fn run(&mut self) {
        let arena = Arena::new();
        let parser = parser::parser(&arena);
        let mapper = LineMapper::new(&self.source);
        match parser.parse(self.source.as_str()) {
            Ok(ast) => {
                let compiled = compiler::compile("initial code".into(), ast, &mapper);
                (self.chunk_callback)(&self.file_name, &compiled.chunk, self.stdout).unwrap();

                if self.run {
                    let mut vm = Vm::initial(compiled, self.stdout);
                    while !vm.done() {
                        vm.step();
                    }
                }
            }
            Err(errors) => {
                (self.error_callback)(&self.file_name, errors, &mapper);
            }
        }
    }
}
