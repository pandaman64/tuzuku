use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    ast::{Ast, AstBody},
    constant::{Constant, Function},
    opcode::{ChunkBuilder, OpCode},
    parser::LineMapper,
};

struct Local {
    ident: String,
    level: usize,
    captured: Cell<bool>,
}

impl Local {
    fn cont() -> Self {
        Self::new("<cont>".into(), 0)
    }

    fn new(ident: String, level: usize) -> Self {
        Self {
            ident,
            level,
            captured: Cell::new(false),
        }
    }

    fn matches(&self, needle: &str) -> bool {
        self.ident == needle
    }
}

enum LookupResult {
    NotFound,
    Upvalue(u8),
    Local(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Upvalue {
    /// The value of the upvalue is stored in the parent function's local variable slots.
    InLocal { index: u8 },
    /// The value of the upvalue is stored in the parent function's upvalue slots.
    InUpvalue { index: u8 },
}

impl Upvalue {
    /// An upvalue whose value is stored in the parent function's locals.
    fn in_local(parent_local_index: u8) -> Self {
        Self::InLocal {
            index: parent_local_index,
        }
    }

    /// An upvalue whose value is stored in the parent function's upvalues.
    fn in_upvalue(parent_upvalue_index: u8) -> Self {
        Self::InUpvalue {
            index: parent_upvalue_index,
        }
    }
}

struct Compiler<'parent> {
    builder: ChunkBuilder,
    /// The list of locals visible by the compiling block, sorted by level.
    locals: Vec<Local>,
    /// The current level of the locals.
    current_level: usize,
    upvalues: RefCell<Vec<Upvalue>>,
    parent: Option<&'parent Compiler<'parent>>,
}

impl Default for Compiler<'_> {
    fn default() -> Self {
        Self::new(None)
    }
}

impl<'parent> Compiler<'parent> {
    fn new(parent: Option<&'parent Compiler<'parent>>) -> Self {
        let mut this = Self {
            builder: ChunkBuilder::default(),
            locals: vec![Local::cont()],
            current_level: 0,
            upvalues: RefCell::new(vec![]),
            parent,
        };
        this.begin_scope();
        this
    }

    fn with_parent(parameters: &[String], parent: &'parent Compiler<'parent>) -> Self {
        let mut this = Self::new(Some(parent));
        for param in parameters.iter() {
            this.push_local(param);
        }
        this
    }

    /// Look up the given identifier from the local variables slots of this function.
    ///
    /// Returns `Some(index)` when a local variable with the same name is found.
    /// Returns `None` when not found.
    fn lookup_local(&self, ident: &str) -> Option<u8> {
        // TODO: handle errors when index overflows
        self.locals
            .iter()
            .rposition(|local| local.matches(ident))
            .map(|index| u8::try_from(index).unwrap())
    }

    fn mark_captured(&self, index: u8) {
        self.locals[usize::from(index)].captured.set(true);
    }

    /// Push the upvalue to this function's upvalue slots, and returns its index in the slots.
    fn push_upvalue(&self, upvalue: Upvalue) -> u8 {
        let mut upvalues = self.upvalues.borrow_mut();

        // If the upvalue is already pushed, return it.
        if let Some(index) = upvalues.iter().position(|u| *u == upvalue) {
            return u8::try_from(index).unwrap();
        }

        let index = upvalues.len();
        upvalues.push(upvalue);
        // TODO: handle overflow
        u8::try_from(index).unwrap()
    }

    /// Look up the given identifier from the ancestors as an upvalue.
    /// Returns the index in this function's upvalue slots.
    ///
    /// This function searches the local variable slots of the parent function and ancestors,
    /// and marks it as captured when found.
    // Allows clippy::manual_map as we want to document each branch.
    #[allow(clippy::manual_map)]
    fn lookup_upvalue(&self, ident: &str) -> Option<u8> {
        // TODO: handle errors when index overflows
        let parent = self.parent?;

        if let Some(parent_local_index) = parent.lookup_local(ident) {
            // The identifier is found in the direct parent's local variable slots,
            // so we'll look up from them.
            parent.mark_captured(parent_local_index);
            Some(self.push_upvalue(Upvalue::in_local(parent_local_index)))
        } else if let Some(parent_upvalue_index) = parent.lookup_upvalue(ident) {
            // The identifier is not found in the direct parent's local variable slots,
            // but found in the upvalue slots (i.e. the identifier comes from the indirect ancestor).
            // In this case, the parent function captures it as an upvalue (by calling `look_upvalue` recursively),
            // and this function look up from the parent's upvalue slots.
            Some(self.push_upvalue(Upvalue::in_upvalue(parent_upvalue_index)))
        } else {
            // The identifier is not found in any of the ancestors.
            None
        }
    }

    fn lookup(&self, ident: &str) -> LookupResult {
        if let Some(local_index) = self.lookup_local(ident) {
            LookupResult::Local(local_index)
        } else if let Some(upvalue_index) = self.lookup_upvalue(ident) {
            LookupResult::Upvalue(upvalue_index)
        } else {
            LookupResult::NotFound
        }
    }

    fn build(mut self, name: String) -> Function {
        Function::new(
            name,
            Rc::new(self.builder.build()),
            self.upvalues.into_inner().len(),
        )
    }

    fn begin_scope(&mut self) {
        self.current_level += 1;
    }

    fn end_scope(&mut self, line: usize) {
        // We emit OP_POP or OP_CLOSE_UPVALUE for each locals in the current scope.
        while let Some(last_local) = self.locals.last() {
            if last_local.level < self.current_level {
                break;
            }

            if last_local.captured.get() {
                self.builder.push_op(OpCode::CloseUpvalue, line);
            } else {
                self.builder.push_op(OpCode::Pop, line);
            }

            self.locals.pop();
        }
        self.current_level -= 1;
    }

    fn push_local(&mut self, ident: &str) {
        self.locals
            .push(Local::new(ident.into(), self.current_level));
    }

    fn emit_set(&mut self, ident: &str, line: usize) {
        match self.lookup(ident) {
            LookupResult::NotFound => {
                let index = self.builder.push_constant(Constant::String(ident.into()));
                self.builder.push_op(OpCode::SetGlobal, line);
                self.builder.push_u8(index, line);
            }
            LookupResult::Upvalue(index) => {
                self.builder.push_op(OpCode::SetUpvalue, line);
                self.builder.push_u8(index, line);
            }
            LookupResult::Local(index) => {
                self.builder.push_op(OpCode::SetLocal, line);
                self.builder.push_u8(index, line);
            }
        }
    }

    fn push_binop(&mut self, opcode: OpCode, lhs: Ast<'_>, rhs: Ast<'_>, mapper: &LineMapper) {
        self.push(lhs, mapper);
        self.push(rhs, mapper);
        self.builder.push_op(opcode, mapper.find(lhs.span.start));
    }

    fn push(&mut self, ast: Ast<'_>, mapper: &LineMapper) {
        let start_line = mapper.find(ast.span.start);
        let end_line = mapper.find(ast.span.end);
        match ast.body {
            AstBody::Number(number) => {
                let index = self.builder.push_constant(Constant::Number(*number));
                self.builder.push_op(OpCode::Constant, start_line);
                self.builder.push_u8(index, start_line);
            }
            AstBody::String(string) => {
                let index = self.builder.push_constant(Constant::String(string.clone()));
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
                    let index = self.builder.push_constant(Constant::String(ident.clone()));
                    self.builder.push_op(OpCode::GetGlobal, start_line);
                    self.builder.push_u8(index, start_line);
                }
                LookupResult::Local(index) => {
                    self.builder.push_op(OpCode::GetLocal, start_line);
                    self.builder.push_u8(index, start_line);
                }
                LookupResult::Upvalue(index) => {
                    self.builder.push_op(OpCode::GetUpvalue, start_line);
                    self.builder.push_u8(index, start_line);
                }
            },
            AstBody::VarDecl { ident, initializer } => {
                if self.parent.is_some() {
                    // Treat the var declaration as local only if it's in a function.
                    self.push_local(ident);

                    // We allocate the slot for the local variable by pushing nil.
                    self.builder.push_op(OpCode::Nil, start_line);

                    // And then, emit SET_LOCAL if the declaration has an initializer.
                    if let Some(initializer) = *initializer {
                        self.push(initializer, mapper);
                        self.emit_set(ident, start_line);
                    }
                } else {
                    // If we declare a global variable, then we emit SET_GLOBAL without
                    // allocating a slot for it.
                    match *initializer {
                        Some(initializer) => self.push(initializer, mapper),
                        None => self.builder.push_op(OpCode::Nil, start_line),
                    }
                    self.emit_set(ident, start_line);
                }
            }
            AstBody::FunDecl {
                ident,
                parameters,
                body,
            } => {
                let mut fun_compiler = Compiler::with_parent(parameters, self);
                for stmt in body.iter() {
                    fun_compiler.push(*stmt, mapper);
                }
                // TODO: handle explicit return
                fun_compiler.end_scope(end_line);
                fun_compiler.builder.push_op(OpCode::Nil, end_line);
                fun_compiler.builder.push_op(OpCode::Return, end_line);
                let function = fun_compiler.build(ident.into());

                let fun_const_index = self.builder.push_constant(Constant::Function(function));
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

pub(crate) fn compile(name: String, ast: Ast<'_>, mapper: &LineMapper) -> Function {
    let mut compiler = Compiler::default();
    compiler.push(ast, mapper);
    // TODO: ここにend_scopeが必要なのが気に食わない
    compiler.end_scope(mapper.find(ast.span.end));
    compiler.build(name)
}
