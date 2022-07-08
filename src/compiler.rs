use crate::{
    ast::Ast,
    opcode::{Chunk, ChunkBuilder, OpCode},
    value::Value,
};

pub(crate) fn compile(ast: Ast) -> Chunk {
    let mut builder = ChunkBuilder::default();

    match ast {
        Ast::Print(content) => {
            let index = builder.push_constant(Value::String(content));
            builder.push_op(OpCode::Constant, 1);
            builder.push_u8(index, 1);
            builder.push_op(OpCode::Print, 1);
        }
    }

    builder.build()
}
