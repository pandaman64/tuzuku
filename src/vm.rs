use std::io::Write;

use crate::{
    opcode::{Chunk, OpCode},
    value::Value,
};

use num_traits::FromPrimitive;

pub(crate) struct Vm<'stdout> {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
    stdout: &'stdout mut (dyn Write + 'stdout),
}

impl<'stdout> Vm<'stdout> {
    pub(crate) fn new(chunk: Chunk, stdout: &'stdout mut (dyn Write + 'stdout)) -> Self {
        Vm {
            chunk,
            ip: 0,
            stack: vec![],
            stdout,
        }
    }

    pub(crate) fn done(&self) -> bool {
        self.ip >= self.chunk.code().len()
    }

    pub(crate) fn step(&mut self) {
        match OpCode::from_u8(self.chunk.code()[self.ip]) {
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
        }
    }
}
