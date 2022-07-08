use crate::{
    ast::{Ast, AstBody},
    opcode::{Chunk, ChunkBuilder, OpCode},
    parser::LineMapper,
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

    fn push_binop(&mut self, opcode: OpCode, lhs: Ast<'_>, rhs: Ast<'_>, mapper: &LineMapper) {
        self.push(lhs, mapper);
        self.push(rhs, mapper);
        self.builder.push_op(opcode, mapper.find(lhs.span.start));
    }

    fn push(&mut self, ast: Ast<'_>, mapper: &LineMapper) {
        let start_line = mapper.find(ast.span.start);
        match ast.body {
            AstBody::Number(number) => {
                let index = self.builder.push_constant(Value::Number(*number));
                self.builder.push_op(OpCode::Constant, start_line);
                self.builder.push_u8(index, start_line);
            }
            AstBody::String(string) => {
                let index = self.builder.push_constant(Value::String(string.clone()));
                self.builder.push_op(OpCode::Constant, start_line);
                self.builder.push_u8(index, start_line);
            }
            AstBody::Print(expr) => {
                self.push(*expr, mapper);
                self.builder.push_op(OpCode::Print, start_line);
            }
            AstBody::Add(lhs, rhs) => self.push_binop(OpCode::Add, *lhs, *rhs, mapper),
            AstBody::Sub(lhs, rhs) => self.push_binop(OpCode::Sub, *lhs, *rhs, mapper),
            AstBody::Mul(lhs, rhs) => self.push_binop(OpCode::Mul, *lhs, *rhs, mapper),
            AstBody::Div(lhs, rhs) => self.push_binop(OpCode::Div, *lhs, *rhs, mapper),
        }
    }
}

pub(crate) fn compile(ast: Ast<'_>, mapper: &LineMapper) -> Chunk {
    let mut compiler = Compiler::default();
    compiler.push(ast, mapper);
    compiler.build()
}
