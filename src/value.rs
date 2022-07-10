use std::rc::Rc;

use crate::opcode::Chunk;

#[derive(Clone)]
pub(crate) struct Continuation {
    /// The chunk to execute.
    chunk: Rc<Chunk>,
    /// The instruction pointer.
    ip: usize,
    /// The starting point in the stack.
    fp: usize,
}

impl Continuation {
    pub(crate) fn new(chunk: Rc<Chunk>) -> Self {
        Self {
            chunk,
            ip: 0,
            fp: 0,
        }
    }

    pub(crate) fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    pub(crate) fn fp(&self) -> usize {
        self.fp
    }

    pub(crate) fn code(&self, increment: usize) -> u8 {
        self.chunk.code()[self.ip + increment]
    }

    pub(crate) fn current_code(&self) -> u8 {
        self.code(0)
    }

    pub(crate) fn constant(&self, index: u8) -> &Value {
        &self.chunk.constants()[usize::from(index)]
    }

    pub(crate) fn done(&self) -> bool {
        self.ip >= self.chunk.code().len()
    }

    pub(crate) fn advance(&mut self, increment: usize) {
        self.ip += increment;
    }
}

#[derive(Clone)]
pub(crate) enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Function { name: String, chunk: Rc<Chunk> },
    Return(Continuation),
}

impl Value {
    pub(crate) fn display(&self) -> String {
        match self {
            Value::Nil => "<nil>".to_string(),
            Value::Boolean(b) => format!("<{}>", b),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Function { name, .. } => format!("<function {}>", name),
            Value::Return(Continuation { ip, fp, .. }) => format!("<return ip={} fp={}>", ip, fp),
        }
    }
}
