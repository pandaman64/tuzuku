use num_derive::FromPrimitive;
use num_traits::FromPrimitive as _;

use crate::value::Value;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive)]
pub(crate) enum OpCode {
    // Constants
    Nil,
    True,
    False,
    Constant,
    // Consumers
    Print,
    Pop,
    // Binary operators
    Add,
    Sub,
    Mul,
    Div,
    // Global
    GetGlobal,
    SetGlobal,
    // Local
    GetLocal,
    SetLocal,
}

pub(crate) struct Chunk {
    code: Box<[u8]>,
    lines: Box<[usize]>,
    constants: Box<[Value]>,
}

impl Chunk {
    pub(crate) fn code(&self) -> &[u8] {
        &self.code
    }

    pub(crate) fn constants(&self) -> &[Value] {
        &self.constants
    }

    fn print_simple(&self, name: &str) -> usize {
        eprintln!(" {:-14} |", name);
        1
    }

    fn print_constant(&self, offset: usize, name: &str) -> usize {
        let index = self.code[offset + 1];
        let constant = &self.constants[usize::from(index)];
        eprintln!(" {:-14} | {}", name, constant.display());
        2
    }

    pub(crate) fn print(&self) {
        eprintln!(" offset | line | {:-14} | constants ", "opcode");
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
                Some(OpCode::Constant) => self.print_constant(offset, "OP_CONSTANT"),
                Some(OpCode::Add) => self.print_simple("OP_ADD"),
                Some(OpCode::Sub) => self.print_simple("OP_SUB"),
                Some(OpCode::Mul) => self.print_simple("OP_MUL"),
                Some(OpCode::Div) => self.print_simple("OP_DIV"),
                Some(OpCode::GetGlobal) => self.print_constant(offset, "OP_GET_GLOBAL"),
                Some(OpCode::SetGlobal) => self.print_constant(offset, "OP_SET_GLOBAL"),
                Some(OpCode::GetLocal) => self.print_constant(offset, "OP_GET_LOCAL"),
                Some(OpCode::SetLocal) => self.print_constant(offset, "OP_SET_LOCAL"),
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
