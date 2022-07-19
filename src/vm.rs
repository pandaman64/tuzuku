use std::{collections::HashMap, io::Write};

use crate::{
    allocator::LEAKING_ALLOCATOR,
    constant::{self, Constant},
    opcode::OpCode,
    value::{Closure, Continuation, Value},
};

use num_traits::FromPrimitive;

#[derive(Default)]
struct Global {
    definitions: HashMap<String, Value>,
}

pub(crate) struct Vm<'stdout> {
    /// The current continuation to run the rest of the program.
    continuation: Continuation,
    global: Global,
    stdout: &'stdout mut (dyn Write + 'stdout),
}

impl<'stdout> Vm<'stdout> {
    pub(crate) fn initial(
        function: constant::Function,
        stdout: &'stdout mut (dyn Write + 'stdout),
    ) -> Self {
        // SAFETY: We pass a valid continuation object.
        let continuation = unsafe {
            Continuation::initial(LEAKING_ALLOCATOR.alloc(Closure::free(function.into())))
        };
        Vm {
            continuation,
            global: Global::default(),
            stdout,
        }
    }

    pub(crate) fn done(&self) -> bool {
        self.continuation.done()
    }

    fn binop(&mut self, op: fn(f64, f64) -> f64) {
        let rhs = self.continuation.stack_mut().pop().unwrap();
        let lhs = self.continuation.stack_mut().pop().unwrap();

        match (lhs, rhs) {
            (Value::Number(lhs), Value::Number(rhs)) => {
                self.continuation
                    .stack_mut()
                    .push(Value::Number(op(lhs, rhs)));
                self.continuation.advance(1);
            }
            _ => panic!("bad type"),
        }
    }

    fn call(&mut self, arguments_len: u8) {
        let callee = self.continuation.call(arguments_len);
        // TODO: the safety of this block relies on the validity of the callee in the stack.
        unsafe {
            let function = callee.as_ref().function();
            function
                .chunk()
                .write(function.name(), self.stdout)
                .unwrap();
        }
    }

    pub(crate) fn step(&mut self) {
        let opcode = OpCode::from_u8(self.continuation.current_code());
        match opcode {
            None => panic!("unknown opcode"),
            Some(OpCode::Nil) => {
                self.continuation.stack_mut().push(Value::Nil);
                self.continuation.advance(1);
            }
            Some(OpCode::True) => {
                self.continuation.stack_mut().push(Value::Boolean(true));
                self.continuation.advance(1);
            }
            Some(OpCode::False) => {
                self.continuation.stack_mut().push(Value::Boolean(false));
                self.continuation.advance(1);
            }
            Some(OpCode::Pop) => {
                self.continuation.stack_mut().pop().unwrap();
                self.continuation.advance(1);
            }
            Some(OpCode::Print) => {
                let value = self.continuation.stack_mut().pop().unwrap();
                writeln!(self.stdout, "{}", value.display()).unwrap();
                self.continuation.advance(1);
            }
            Some(OpCode::Call) => {
                let arguments_len = self.continuation.code(1);
                // Return to the next opcode of OP_CALL.
                self.continuation.advance(2);

                self.call(arguments_len);
            }
            Some(OpCode::Return) => {
                self.continuation.perform_return();
            }
            Some(OpCode::Constant) => {
                let index = self.continuation.code(1);
                let constant = self.continuation.constant(index).clone();
                self.continuation.stack_mut().push(constant.into());
                self.continuation.advance(2);
            }
            Some(OpCode::Add) => self.binop(|lhs, rhs| lhs + rhs),
            Some(OpCode::Sub) => self.binop(|lhs, rhs| lhs - rhs),
            Some(OpCode::Mul) => self.binop(|lhs, rhs| lhs * rhs),
            Some(OpCode::Div) => self.binop(|lhs, rhs| lhs / rhs),
            Some(OpCode::GetGlobal) => {
                let index = self.continuation.code(1);
                let constant = self.continuation.constant(index);
                match constant {
                    Constant::String(name) => {
                        let value = self.global.definitions[name].clone();
                        self.continuation.stack_mut().push(value);
                        self.continuation.advance(2);
                    }
                    _ => unreachable!("compile error: OP_GET_GLOBAL takes a string constant"),
                }
            }
            Some(OpCode::SetGlobal) => {
                let index = self.continuation.code(1);
                let constant = self.continuation.constant(index).clone();
                match constant {
                    Constant::String(name) => {
                        let value = self.continuation.stack_mut().pop().unwrap();
                        self.global.definitions.insert(name, value);
                        self.continuation.advance(2);
                    }
                    _ => unreachable!("compile error: OP_SET_GLOBAL takes a string constant"),
                }
            }
            Some(OpCode::GetLocal) => {
                let offset = self.continuation.code(1);
                let value = self.continuation.stack_mut().get_local(offset);
                self.continuation.stack_mut().push(value);
                self.continuation.advance(2);
            }
            Some(OpCode::SetLocal) => {
                let offset = self.continuation.code(1);
                let value = self.continuation.stack_mut().pop().unwrap();
                self.continuation.stack_mut().set_local(offset, value);
                self.continuation.advance(2);
            }
            Some(OpCode::CloseUpvalue) => todo!(),
            Some(OpCode::GetUpvalue) => todo!(),
            Some(OpCode::SetUpvalue) => todo!(),
        }
    }
}
