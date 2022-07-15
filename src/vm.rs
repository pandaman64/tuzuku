use std::{collections::HashMap, io::Write, rc::Rc};

use crate::{
    opcode::{Chunk, OpCode},
    value::{Continuation, Value},
};

use num_traits::FromPrimitive;

#[derive(Default)]
struct Global {
    definitions: HashMap<String, Value>,
}

pub(crate) struct Vm<'stdout> {
    continuation: Continuation,
    stack: Vec<Value>,
    global: Global,
    stdout: &'stdout mut (dyn Write + 'stdout),
}

impl<'stdout> Vm<'stdout> {
    pub(crate) fn new(chunk: Chunk, stdout: &'stdout mut (dyn Write + 'stdout)) -> Self {
        Vm {
            continuation: Continuation::new(Rc::new(chunk)),
            stack: vec![],
            global: Global::default(),
            stdout,
        }
    }

    pub(crate) fn done(&self) -> bool {
        self.continuation.done()
    }

    fn binop(&mut self, op: fn(f64, f64) -> f64) {
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.pop().unwrap();

        match (lhs, rhs) {
            (Value::Number(lhs), Value::Number(rhs)) => {
                self.stack.push(Value::Number(op(lhs, rhs)));
                self.continuation.advance(1);
            }
            _ => panic!("bad type"),
        }
    }

    fn call(&mut self, arguments_len: u8) {
        let return_continuation = Value::Return(self.continuation.clone());

        let callee_index = self.stack.len() - usize::from(arguments_len) - 1;
        let callee = std::mem::replace(&mut self.stack[callee_index], return_continuation);
        match callee {
            Value::Function { name, chunk } => {
                self.continuation = Continuation::new(chunk);
                self.continuation.chunk().write(&name, self.stdout).unwrap();
            }
            _ => todo!("type check function callee"),
        }
    }

    pub(crate) fn step(&mut self) {
        let opcode = OpCode::from_u8(self.continuation.current_code());
        match opcode {
            None => panic!("unknown opcode"),
            Some(OpCode::Nil) => {
                self.stack.push(Value::Nil);
                self.continuation.advance(1);
            }
            Some(OpCode::True) => {
                self.stack.push(Value::Boolean(true));
                self.continuation.advance(1);
            }
            Some(OpCode::False) => {
                self.stack.push(Value::Boolean(false));
                self.continuation.advance(1);
            }
            Some(OpCode::Pop) => {
                self.stack.pop().unwrap();
                self.continuation.advance(1);
            }
            Some(OpCode::CloseUpvalue) => todo!(),
            Some(OpCode::Print) => {
                let value = self.stack.pop().unwrap();
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
                let return_value = self.stack.pop().unwrap();
                let continuation = {
                    // The stack frame for this function call.
                    let mut frame = self.stack.drain(self.continuation.fp()..);
                    frame.next().unwrap()
                };
                match continuation {
                    Value::Return(continuation) => {
                        self.continuation = continuation;
                        self.stack.push(return_value);
                    }
                    _ => todo!("type error at OP_RETURN"),
                }
            }
            Some(OpCode::Constant) => {
                let index = self.continuation.code(1);
                let value = self.continuation.constant(index).clone();
                self.stack.push(value);
                self.continuation.advance(2);
            }
            Some(OpCode::Add) => self.binop(|lhs, rhs| lhs + rhs),
            Some(OpCode::Sub) => self.binop(|lhs, rhs| lhs - rhs),
            Some(OpCode::Mul) => self.binop(|lhs, rhs| lhs * rhs),
            Some(OpCode::Div) => self.binop(|lhs, rhs| lhs / rhs),
            Some(OpCode::GetGlobal) => {
                let index = self.continuation.code(1);
                let value = self.continuation.constant(index);
                match value {
                    Value::String(name) => {
                        let value = self.global.definitions[name].clone();
                        self.stack.push(value);
                        self.continuation.advance(2);
                    }
                    _ => unreachable!("compile error: OP_GET_GLOBAL takes a string constant"),
                }
            }
            Some(OpCode::SetGlobal) => {
                let index = self.continuation.code(1);
                let value = self.continuation.constant(index);
                match value {
                    Value::String(name) => {
                        let value = self.stack.pop().unwrap();
                        self.global.definitions.insert(name.clone(), value);
                        self.continuation.advance(2);
                    }
                    _ => unreachable!("compile error: OP_SET_GLOBAL takes a string constant"),
                }
            }
            Some(OpCode::GetLocal) => {
                let offset = self.continuation.code(1);
                let value = self.stack[self.continuation.fp() + usize::from(offset)].clone();
                self.stack.push(value);
                self.continuation.advance(2);
            }
            Some(OpCode::SetLocal) => {
                let offset = self.continuation.code(1);
                let value = self.stack.pop().unwrap();
                self.stack[self.continuation.fp() + usize::from(offset)] = value;
                self.continuation.advance(2);
            }
            Some(OpCode::GetUpvalue) => todo!(),
            Some(OpCode::SetUpvalue) => todo!(),
        }
    }
}
