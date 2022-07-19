use std::rc::Rc;

use crate::opcode::Chunk;

/// The compile-time representation of a function.
#[derive(Clone)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) chunk: Rc<Chunk>,
    pub(crate) upvalues: usize,
}

impl Function {
    pub(crate) fn new(name: String, chunk: Rc<Chunk>, upvalues: usize) -> Self {
        Self {
            name,
            chunk,
            upvalues,
        }
    }
}

#[derive(Clone)]
pub(crate) enum Constant {
    Number(f64),
    String(String),
    Function(Function),
}

impl Constant {
    pub(crate) fn display(&self) -> String {
        match self {
            Constant::Number(n) => n.to_string(),
            Constant::String(s) => s.clone(),
            Constant::Function(f) => format!("<function {}>", f.name),
        }
    }
}
