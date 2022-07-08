use std::{collections::HashMap, io::Write};

use crate::{
    opcode::{Chunk, OpCode},
    value::Value,
};

use num_traits::FromPrimitive;

#[derive(Default)]
struct Global {
    definitions: HashMap<String, Value>,
}

pub(crate) struct Vm<'stdout> {
    chunk: Chunk,
    ip: usize,
    fp: usize,
    stack: Vec<Value>,
    global: Global,
    stdout: &'stdout mut (dyn Write + 'stdout),
}

impl<'stdout> Vm<'stdout> {
    pub(crate) fn new(chunk: Chunk, stdout: &'stdout mut (dyn Write + 'stdout)) -> Self {
        Vm {
            chunk,
            ip: 0,
            fp: 0,
            stack: vec![],
            global: Global::default(),
            stdout,
        }
    }

    pub(crate) fn done(&self) -> bool {
        self.ip >= self.chunk.code().len()
    }

    fn binop(&mut self, op: fn(f64, f64) -> f64) {
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.pop().unwrap();

        match (lhs, rhs) {
            (Value::Number(lhs), Value::Number(rhs)) => {
                self.stack.push(Value::Number(op(lhs, rhs)));
                self.ip += 1;
            }
            _ => panic!("bad type"),
        }
    }

    pub(crate) fn step(&mut self) {
        let opcode = OpCode::from_u8(self.chunk.code()[self.ip]);
        match opcode {
            None => panic!("unknown opcode"),
            Some(OpCode::Nil) => {
                self.stack.push(Value::Nil);
                self.ip += 1;
            }
            Some(OpCode::True) => {
                self.stack.push(Value::Boolean(true));
                self.ip += 1;
            }
            Some(OpCode::False) => {
                self.stack.push(Value::Boolean(false));
                self.ip += 1;
            }
            Some(OpCode::Pop) => {
                self.stack.pop();
                self.ip += 1;
            }
            Some(OpCode::Print) => {
                let value = self.stack.pop().unwrap();
                writeln!(self.stdout, "{}", value.display()).unwrap();
                self.ip += 1;
            }
            Some(OpCode::Constant) => {
                let index = self.chunk.code()[self.ip + 1];
                let value = self.chunk.constants()[usize::from(index)].clone();
                self.stack.push(value);
                self.ip += 2;
            }
            Some(OpCode::Add) => self.binop(|lhs, rhs| lhs + rhs),
            Some(OpCode::Sub) => self.binop(|lhs, rhs| lhs - rhs),
            Some(OpCode::Mul) => self.binop(|lhs, rhs| lhs * rhs),
            Some(OpCode::Div) => self.binop(|lhs, rhs| lhs / rhs),
            Some(OpCode::GetGlobal) => {
                let index = self.chunk.code()[self.ip + 1];
                let value = &self.chunk.constants()[usize::from(index)];
                match value {
                    Value::String(name) => {
                        let value = self.global.definitions[name].clone();
                        self.stack.push(value);
                        self.ip += 2;
                    }
                    _ => unreachable!("compile error: OP_GET_GLOBAL takes a string constant"),
                }
            }
            Some(OpCode::SetGlobal) => {
                let index = self.chunk.code()[self.ip + 1];
                let value = &self.chunk.constants()[usize::from(index)];
                match value {
                    Value::String(name) => {
                        let value = self.stack.pop().unwrap();
                        self.global.definitions.insert(name.clone(), value);
                        self.ip += 2;
                    }
                    _ => unreachable!("compile error: OP_SET_GLOBAL takes a string constant"),
                }
            }
            Some(OpCode::GetLocal) => {
                let offset = self.chunk.code()[self.ip + 1];
                let value = self.stack[self.fp + usize::from(offset)].clone();
                self.stack.push(value);
            }
            Some(OpCode::SetLocal) => {
                let offset = self.chunk.code()[self.ip + 1];
                let value = self.stack.pop().unwrap();
                self.stack[self.fp + usize::from(offset)] = value;
            }
        }
    }
}
