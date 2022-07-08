use crate::{
    ast::{Ast, AstBody},
    opcode::{Chunk, ChunkBuilder, OpCode},
    value::Value,
};

#[derive(Default)]
struct Compiler {
    builder: ChunkBuilder,
}

impl Compiler {
    fn build(mut self) -> Chunk {
        self.builder.build()
    }

    fn push_binop(&mut self, opcode: OpCode, lhs: Ast<'_>, rhs: Ast<'_>, line: usize) {
        self.push(lhs);
        self.push(rhs);
        self.builder.push_op(opcode, line);
    }

    fn push(&mut self, ast: Ast<'_>) {
        match ast {
            AstBody::Number(number) => {
                let index = self.builder.push_constant(Value::Number(*number));
                self.builder.push_op(OpCode::Constant, 1);
                self.builder.push_u8(index, 1);
            }
            AstBody::String(string) => {
                let index = self.builder.push_constant(Value::String(string.clone()));
                self.builder.push_op(OpCode::Constant, 1);
                self.builder.push_u8(index, 1);
            }
            AstBody::Print(expr) => {
                self.push(expr);
                self.builder.push_op(OpCode::Print, 1);
            }
            AstBody::Add(lhs, rhs) => self.push_binop(OpCode::Add, lhs, rhs, 1),
            AstBody::Sub(lhs, rhs) => self.push_binop(OpCode::Sub, lhs, rhs, 1),
            AstBody::Mul(lhs, rhs) => self.push_binop(OpCode::Mul, lhs, rhs, 1),
            AstBody::Div(lhs, rhs) => self.push_binop(OpCode::Div, lhs, rhs, 1),
        }
    }
}

pub(crate) fn compile(ast: Ast<'_>) -> Chunk {
    let mut compiler = Compiler::default();
    compiler.push(ast);
    compiler.build()
}
