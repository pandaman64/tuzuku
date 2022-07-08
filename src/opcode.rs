use num_derive::FromPrimitive;
use num_traits::FromPrimitive as _;

use crate::value::Value;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive)]
pub(crate) enum OpCode {
    Nil,
    True,
    False,
    Constant,
    Print,
    Pop,
}

pub(crate) struct Chunk {
    code: Box<[u8]>,
    lines: Box<[usize]>,
    constants: Box<[Value]>,
}

impl Chunk {
    fn print_simple(&self, name: &str) -> usize {
        eprintln!(" {:-12} |", name);
        1
    }

    fn print_constant(&self, offset: usize) -> usize {
        let index = self.code[offset + 1];
        let constant = &self.constants[usize::from(index)];
        eprintln!(" {:-12} | {}", "OP_CONSTANT", constant.display());
        2
    }

    pub(crate) fn print(&self) {
        eprintln!(" offset | line | {:-12} | constants ", "opcode");
        let mut offset = 0;
        while offset < self.code.len() {
            eprint!(" {:06} | {:04} |", offset, self.lines[offset]);

            offset += match OpCode::from_u8(self.code[offset]) {
                None => self.print_simple("OP_UNKNOWN"),
                Some(OpCode::Nil) => self.print_simple("OP_NIL"),
                Some(OpCode::True) => self.print_simple("OP_TRUE"),
                Some(OpCode::False) => self.print_simple("OP_FALSE"),
                Some(OpCode::Pop) => self.print_simple("OP_POP"),
                Some(OpCode::Print) => self.print_simple("OP_PRINT"),
                Some(OpCode::Constant) => self.print_constant(offset),
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct ChunkBuilder {
    code: Vec<u8>,
    lines: Vec<usize>,
    constants: Vec<Value>,
}

impl ChunkBuilder {
    pub(crate) fn push_op(&mut self, opcode: OpCode, line: usize) {
        self.push_u8(opcode as u8, line);
    }

    pub(crate) fn push_u8(&mut self, code: u8, line: usize) {
        self.code.push(code);
        self.lines.push(line);
    }

    pub(crate) fn push_constant(&mut self, constant: Value) -> u8 {
        let index = self.constants.len();
        self.constants.push(constant);
        u8::try_from(index).unwrap()
    }

    pub(crate) fn build(&mut self) -> Chunk {
        let this = std::mem::take(self);
        Chunk {
            code: this.code.into_boxed_slice(),
            lines: this.lines.into_boxed_slice(),
            constants: this.constants.into_boxed_slice(),
        }
    }
}
