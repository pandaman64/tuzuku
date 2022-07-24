use chumsky::Parser;
use typed_arena::Arena;

use crate::{
    compiler,
    parser::{self, LineMapper},
    side_effect::SideEffectHandler,
    vm::Vm,
};

pub(crate) struct Driver<'handler> {
    pub(crate) file_name: String,
    pub(crate) source: String,
    pub(crate) run: bool,
    pub(crate) handler: &'handler mut (dyn SideEffectHandler + 'handler),
}

impl Driver<'_> {
    pub(crate) fn run(&mut self) {
        let arena = Arena::new();
        let parser = parser::parser(&arena);
        let mapper = LineMapper::new(&self.source);
        match parser.parse(self.source.as_str()) {
            Ok(ast) => {
                let compiled =
                    compiler::compile(format!("{}_initial_code", self.file_name), ast, &mapper);

                if self.run {
                    let mut vm = Vm::initial(compiled, self.handler);
                    while !vm.done() {
                        vm.step();
                    }
                }
            }
            Err(errors) => self
                .handler
                .compile_error(&self.file_name, errors, &mapper)
                .unwrap(),
        }
    }
}
