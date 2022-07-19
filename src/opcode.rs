use std::io;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive as _;

use crate::constant::Constant;

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
    CloseUpvalue,
    Call,
    Return,
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
    // Upvalue
    GetUpvalue,
    SetUpvalue,
    Closure,
}

pub(crate) struct Chunk {
    code: Box<[u8]>,
    lines: Box<[usize]>,
    constants: Box<[Constant]>,
}

impl Chunk {
    pub(crate) fn code(&self) -> &[u8] {
        &self.code
    }

    pub(crate) fn constants(&self) -> &[Constant] {
        &self.constants
    }

    fn print_simple(&self, writer: &mut dyn io::Write, name: &str) -> io::Result<usize> {
        writeln!(writer, " {:-14} |", name)?;
        Ok(1)
    }

    fn print_constant(
        &self,
        writer: &mut dyn io::Write,
        offset: usize,
        name: &str,
    ) -> io::Result<usize> {
        let index = self.code[offset + 1];
        let constant = &self.constants[usize::from(index)];
        writeln!(writer, " {:-14} | {}", name, constant.display())?;
        Ok(2)
    }

    fn print_immediate(
        &self,
        writer: &mut dyn io::Write,
        offset: usize,
        name: &str,
    ) -> io::Result<usize> {
        let immediate = self.code[offset + 1];
        writeln!(writer, " {:-14} | {}", name, immediate)?;
        Ok(2)
    }

    fn print_closure(&self, writer: &mut dyn io::Write, offset: usize) -> io::Result<usize> {
        // OP_CLOSURE is a variable-length opcode where
        // | OP_CLOSURE | # of upvalues | (#1) true if values comes from local of the parent | (#1) the index in the local/upvalue | ... |
        let upvalues = usize::from(self.code[offset + 1]);

        writeln!(writer, " {:-14} | {}", "OP_CLOSURE", upvalues)?;
        for i in 0..upvalues {
            let is_local = self.code[offset + 1 + 2 * i] > 0;
            let index = self.code[offset + 1 + 2 * i + 1];
            writeln!(
                writer,
                " {:6} | {:4} | {:-14} | {} ({})",
                "",
                "",
                "",
                index,
                if is_local { "local" } else { "upvalue" }
            )?;
        }

        Ok(2 + 2 * upvalues)
    }

    pub(crate) fn write(&self, name: &str, writer: &mut dyn io::Write) -> io::Result<()> {
        writeln!(writer, "==== {} ====", name)?;
        writeln!(writer, " offset | line | {:-14} | constants", "opcode")?;
        let mut offset = 0;
        while offset < self.code.len() {
            write!(writer, " {:06} | {:04} |", offset, self.lines[offset])?;

            offset += match OpCode::from_u8(self.code[offset]) {
                None => self.print_simple(writer, "OP_UNKNOWN")?,
                Some(OpCode::Nil) => self.print_simple(writer, "OP_NIL")?,
                Some(OpCode::True) => self.print_simple(writer, "OP_TRUE")?,
                Some(OpCode::False) => self.print_simple(writer, "OP_FALSE")?,
                Some(OpCode::Pop) => self.print_simple(writer, "OP_POP")?,
                Some(OpCode::CloseUpvalue) => self.print_simple(writer, "OP_CLOSE_UPVALUE")?,
                Some(OpCode::Print) => self.print_simple(writer, "OP_PRINT")?,
                Some(OpCode::Call) => self.print_immediate(writer, offset, "OP_CALL")?,
                Some(OpCode::Return) => self.print_simple(writer, "OP_RETURN")?,
                Some(OpCode::Constant) => self.print_constant(writer, offset, "OP_CONSTANT")?,
                Some(OpCode::Add) => self.print_simple(writer, "OP_ADD")?,
                Some(OpCode::Sub) => self.print_simple(writer, "OP_SUB")?,
                Some(OpCode::Mul) => self.print_simple(writer, "OP_MUL")?,
                Some(OpCode::Div) => self.print_simple(writer, "OP_DIV")?,
                Some(OpCode::GetGlobal) => self.print_constant(writer, offset, "OP_GET_GLOBAL")?,
                Some(OpCode::SetGlobal) => self.print_constant(writer, offset, "OP_SET_GLOBAL")?,
                Some(OpCode::GetLocal) => self.print_immediate(writer, offset, "OP_GET_LOCAL")?,
                Some(OpCode::SetLocal) => self.print_immediate(writer, offset, "OP_SET_LOCAL")?,
                Some(OpCode::GetUpvalue) => {
                    self.print_immediate(writer, offset, "OP_GET_UPVALUE")?
                }
                Some(OpCode::SetUpvalue) => {
                    self.print_immediate(writer, offset, "OP_SET_UPVALUE")?
                }
                Some(OpCode::Closure) => self.print_closure(writer, offset)?,
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct ChunkBuilder {
    code: Vec<u8>,
    lines: Vec<usize>,
    constants: Vec<Constant>,
}

impl ChunkBuilder {
    pub(crate) fn push_op(&mut self, opcode: OpCode, line: usize) {
        self.push_u8(opcode as u8, line);
    }

    pub(crate) fn push_u8(&mut self, code: u8, line: usize) {
        self.code.push(code);
        self.lines.push(line);
    }

    pub(crate) fn push_constant(&mut self, constant: Constant) -> u8 {
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
