use std::{cell::Cell, rc::Rc};

use crate::{
    ast::{Ast, AstBody},
    opcode::{Chunk, ChunkBuilder, OpCode},
    parser::LineMapper,
    value::Value,
};

struct Local {
    ident: String,
    captured: Cell<bool>,
}

impl Local {
    fn new(ident: String) -> Self {
        Self {
            ident,
            captured: Cell::new(false),
        }
    }

    fn matches(&self, needle: &str) -> bool {
        self.ident == needle
    }
}

enum LookupResult {
    NotFound,
    Upvalue(usize),
    Current(usize),
}

struct Compiler<'parent> {
    builder: ChunkBuilder,
    locals: Vec<Local>,
    parent: Option<&'parent Compiler<'parent>>,
}

impl Default for Compiler<'_> {
    fn default() -> Self {
        Self {
            builder: ChunkBuilder::default(),
            locals: vec![Local::new("<cont>".into())],
            parent: None,
        }
    }
}

impl<'parent> Compiler<'parent> {
    fn new(parameters: &[String], parent: &'parent Compiler<'parent>) -> Self {
        let mut locals = vec![Local::new("<cont>".into())];
        locals.extend(parameters.iter().map(|param| Local::new(param.into())));
        Self {
            builder: ChunkBuilder::default(),
            locals,
            parent: Some(parent),
        }
    }

    fn lookup(&self, ident: &str) -> LookupResult {
        match self.locals.iter().rposition(|local| local.matches(ident)) {
            Some(index) => LookupResult::Current(index),
            None => match self.parent {
                Some(parent) => match parent.lookup(ident) {
                    LookupResult::NotFound => LookupResult::NotFound,
                    _ => todo!(),
                },
                None => LookupResult::NotFound,
            },
        }
    }

    fn build(mut self) -> Chunk {
        self.builder.build()
    }

    fn is_toplevel(&self) -> bool {
        self.parent.is_none()
    }

    fn push_local(&mut self, ident: &str) -> u8 {
        let index = self.locals.len();
        self.locals.push(Local::new(ident.into()));
        u8::try_from(index).unwrap()
    }

    fn emit_set(&mut self, ident: &str, line: usize) {
        if self.is_toplevel() {
            let index = self.builder.push_constant(Value::String(ident.into()));
            self.builder.push_op(OpCode::SetGlobal, line);
            self.builder.push_u8(index, line);
        } else {
            let index = self.push_local(ident);
            self.builder.push_op(OpCode::SetLocal, line);
            self.builder.push_u8(index, line);
        }
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
            AstBody::Root(stmts) => {
                for stmt in stmts.iter() {
                    self.push(*stmt, mapper);
                }
            }
            AstBody::Assign(ident, expr) => {
                self.push(*expr, mapper);
                self.emit_set(ident, start_line);
            }
            AstBody::Var(ident) => match self.lookup(ident) {
                LookupResult::NotFound => {
                    let index = self.builder.push_constant(Value::String(ident.clone()));
                    self.builder.push_op(OpCode::GetGlobal, start_line);
                    self.builder.push_u8(index, start_line);
                }
                LookupResult::Current(index) => {
                    self.builder.push_op(OpCode::GetLocal, start_line);
                    self.builder
                        .push_u8(u8::try_from(index).unwrap(), start_line);
                }
                LookupResult::Upvalue(_) => todo!(),
            },
            AstBody::FunDecl {
                ident,
                parameters,
                body,
            } => {
                let mut fun_compiler = Compiler::new(parameters, self);
                for stmt in body.iter() {
                    fun_compiler.push(*stmt, mapper);
                }
                let fun_chunk = fun_compiler.build();

                let fun_const_index = self.builder.push_constant(Value::Function {
                    name: ident.into(),
                    chunk: Rc::new(fun_chunk),
                });
                self.builder.push_op(OpCode::Constant, start_line);
                self.builder.push_u8(fun_const_index, start_line);
                self.emit_set(ident, start_line);
            }
            AstBody::Call { callee, arguments } => {
                self.push(*callee, mapper);
                for argument in arguments.iter() {
                    self.push(*argument, mapper);
                }
                self.builder.push_op(OpCode::Call, start_line);
                self.builder
                    .push_u8(u8::try_from(arguments.len()).unwrap(), start_line);
            }
            AstBody::ExprStmt { expr } => {
                self.push(*expr, mapper);
                self.builder.push_op(OpCode::Pop, start_line);
            }
        }
    }
}

pub(crate) fn compile(ast: Ast<'_>, mapper: &LineMapper) -> Chunk {
    let mut compiler = Compiler::default();
    compiler.push(ast, mapper);
    compiler.build()
}
