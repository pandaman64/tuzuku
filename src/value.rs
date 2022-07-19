use std::{ptr::NonNull, rc::Rc};

use crate::{
    allocator::LEAKING_ALLOCATOR,
    constant::{self, Constant},
    opcode::Chunk,
};

const STACK_SIZE: usize = 1024;

#[derive(Clone)]
pub(crate) struct Stack {
    /// The value stack.
    ///
    /// # Invariant
    /// values must be initialized and has STACK_SIZE valid elements indefinitely.
    /// TODO: GC will destory and reclaim the stack once implemented.
    values: NonNull<[Option<Value>]>,
    /// The index at the past one after the end of stack.
    sp: usize,
    /// The starting point of the current function in the stack.
    fp: usize,
}

impl Stack {
    fn empty() -> Self {
        Self {
            values: LEAKING_ALLOCATOR.alloc_array(None, STACK_SIZE),
            sp: 0,
            fp: 0,
        }
    }

    fn check(&self) {
        assert!(self.sp < STACK_SIZE);
        assert!(self.fp <= self.sp);

        #[cfg(debug_assertions)]
        {
            // SAFETY: self.values is initialized.
            unsafe {
                for idx in 0..self.values.len() {
                    let value = self.values.get_unchecked_mut(idx);
                    assert_eq!(idx < self.sp, value.as_ref().is_some())
                }
            }
        }
    }

    pub(crate) fn push(&mut self, value: Value) {
        self.check();
        // TODO: stack overflow
        // SAFETY: self.check() ensures that self.sp points to inside the stack,
        // so it's safe to dereference and assign to it.
        unsafe {
            *self.values.get_unchecked_mut(self.sp).as_mut() = Some(value);
        }
        self.sp += 1;
    }

    pub(crate) fn pop(&mut self) -> Option<Value> {
        self.check();
        // TODO: negative overflow
        self.sp -= 1;
        // SAFETY: self.check() ensures that self.sp points to inside the stack,
        // so it's safe to dereference and assign to it.
        unsafe { std::mem::replace(self.values.get_unchecked_mut(self.sp).as_mut(), None) }
    }

    fn replace_at(&mut self, index: usize, value: Value) -> Value {
        self.check();
        assert!(index < self.sp);
        // SAFETY: self.check() ensures that self.sp points to inside the stack,
        // and index is less than self.sp, so we can dereference at index.
        unsafe {
            let place = self.values.get_unchecked_mut(index).as_mut();
            std::mem::replace(place, Some(value)).unwrap()
        }
    }

    /// Reset sp and drop the values in the following slots.
    fn rewind_sp(&mut self, new_sp: usize) {
        assert!(new_sp < self.sp);

        // SAFETY: self.check() ensures that self.sp points to inside the stack,
        // and new_sp is less than self.sp, so we can dereference between them.
        unsafe {
            for place in self
                .values
                .get_unchecked_mut(new_sp..self.sp)
                .as_mut()
                .iter_mut()
            {
                *place = None;
            }
        }
        self.sp = new_sp;

        self.check();
    }

    fn get_local_ptr(&self, offset: u8) -> NonNull<Option<Value>> {
        self.check();

        let index = self.fp + usize::from(offset);
        assert!(index < self.sp);

        // SAFETY: self.check() ensures that self.sp points to inside the stack,
        // and index is less than self.sp, so we can point to the index.
        unsafe { self.values.get_unchecked_mut(index) }
    }

    pub(crate) fn get_local(&self, offset: u8) -> Value {
        // SAFETY: self.get_local_ptr() returns a pointer to a valid stack slot.
        unsafe { self.get_local_ptr(offset).as_ref().clone().unwrap() }
    }

    pub(crate) fn set_local(&mut self, offset: u8, value: Value) {
        self.check();
        self.replace_at(self.fp + usize::from(offset), value);
    }
}

#[derive(Clone)]
pub(crate) struct Continuation {
    /// The closure to execute.
    ///
    /// # Invariant
    /// The closure must be valid indefinitely.
    /// TODO: GC will destory and reclaim the closure once implemented.
    closure: NonNull<Closure>,
    /// The instruction pointer.
    ip: usize,
    /// The value stack
    stack: Stack,
}

impl Continuation {
    /// Create a continuation at the start of running the program.
    ///
    /// # Safety
    /// The given closure must be valid which is the assumption of the rest of methods.
    pub(crate) unsafe fn initial(closure: NonNull<Closure>) -> Self {
        Self {
            closure,
            stack: Stack::empty(),
            ip: 0,
        }
    }

    fn closure(&self) -> &Closure {
        // SAFETY: the requirement of the constructor permits this read.
        unsafe { self.closure.as_ref() }
    }

    fn function(&self) -> &Function {
        &self.closure().function
    }

    fn chunk(&self) -> &Chunk {
        &self.function().chunk
    }

    pub(crate) fn stack_mut(&mut self) -> &mut Stack {
        &mut self.stack
    }

    pub(crate) fn code(&self, increment: usize) -> u8 {
        self.chunk().code()[self.ip + increment]
    }

    pub(crate) fn current_code(&self) -> u8 {
        self.code(0)
    }

    pub(crate) fn constant(&self, index: u8) -> &Constant {
        &self.chunk().constants()[usize::from(index)]
    }

    pub(crate) fn done(&self) -> bool {
        self.ip >= self.chunk().code().len()
    }

    pub(crate) fn advance(&mut self, increment: usize) {
        self.ip += increment;
    }

    pub(crate) fn display(&self) -> String {
        format!(
            "ip = {}, sp = {}, fp = {}",
            self.ip, self.stack.sp, self.stack.fp
        )
    }

    /// Call a function on the top of the stack.
    pub(crate) fn call(&mut self, arguments_len: u8) -> NonNull<Closure> {
        // NOTE: the stack pointer of the return_continuation is invalid when we return from the function.
        // But, perform_return() adjust it when we actually return to the callee.
        let return_continuation = Value::Return(self.clone());
        let callee_index = self.stack.sp - usize::from(arguments_len) - 1;
        let callee = self.stack.replace_at(callee_index, return_continuation);
        let closure = match callee {
            Value::Function(function) => LEAKING_ALLOCATOR.alloc(Closure::free(function)),
            Value::Closure(closure) => closure,
            _ => todo!("callee is not a function nor a closure"),
        };

        // Jump to the start of the given chunk.
        self.closure = closure;
        self.ip = 0;
        // Shift the frame pointer (stack pointer remains same).
        self.stack.fp = callee_index;

        closure
    }

    /// Run the return procedure.
    ///
    /// It first retrieves the continuation to return, and reset self to it.
    /// Then, it adjusts the stack pointer and push the return value on top of the stack.
    pub(crate) fn perform_return(&mut self) {
        let fp = self.stack.fp;

        let return_value = self.stack.pop().unwrap();
        let continuation = self.stack.get_local(0);
        match continuation {
            Value::Return(continuation) => {
                *self = continuation;
                self.stack.rewind_sp(fp);
                self.stack.push(return_value);
            }
            _ => todo!("The return continuation is not a continuation"),
        }
    }

    /// Get the pointer to the object held by the current function's upvalue at the index.
    fn get_upvalue_ptr(&self, index: u8) -> NonNull<Option<Value>> {
        // TODO: the assumption of safety is that the upvalues stored in the closure are valid.
        unsafe {
            let closure = self.closure();
            let upvalues = closure.upvalues().get_unchecked_mut(usize::from(index));
            upvalues.as_ref().as_ref().pointer
        }
    }

    /// Create a closure on stack.
    pub(crate) fn perform_closure(&mut self) {
        let function = match self.stack.pop().unwrap() {
            Value::Function(function) => function,
            _ => todo!("type error: OP_CLOSURE takes function"),
        };

        let upvalues_len = usize::from(self.code(1));
        let upvalues: Box<[NonNull<Upvalue>]> = (0..upvalues_len)
            .map(|idx| {
                let is_local = self.code(1 + 2 * idx) > 0;
                let index = self.code(1 + 2 * idx + 1);
                let pointer = if is_local {
                    self.stack.get_local_ptr(index)
                } else {
                    self.get_upvalue_ptr(index)
                };
                // TODO: link and cache upvalues
                LEAKING_ALLOCATOR.alloc(Upvalue::open(None, pointer))
            })
            .collect();
        // SAFETY: the pointer is valid.
        let upvalues = unsafe { NonNull::new_unchecked(Box::into_raw(upvalues)) };
        let closure =
            Value::Closure(LEAKING_ALLOCATOR.alloc(Closure::capturing(function, upvalues)));
        self.stack.push(closure);
        self.advance(2 + 2 * upvalues_len);
    }
}

/// The run-time representation of a function.
#[derive(Clone)]
pub(crate) struct Function {
    name: String,
    chunk: Rc<Chunk>,
    /// The number of upvalues.
    upvalues: usize,
}

impl From<constant::Function> for Function {
    fn from(function: constant::Function) -> Self {
        Self::new(function.name, function.chunk, function.upvalues)
    }
}

impl Function {
    pub(crate) fn new(name: String, chunk: Rc<Chunk>, upvalues: usize) -> Self {
        Self {
            name,
            chunk,
            upvalues,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    pub(crate) fn upvalues(&self) -> usize {
        self.upvalues
    }
}

/// The run-time representation of upvalues.
pub(crate) struct Upvalue {
    /// The pointer to the next upvalue.
    ///
    /// The next upvalue must point to a slot in the same stack that has smaller index than this (next.pointer < pointer).
    next: Option<NonNull<Upvalue>>,
    /// The pointer to the pointed value.
    ///
    /// It points to either a slot in a stack or closed of self.
    /// TODO: is it okay to use self-referential pointer?
    pointer: NonNull<Option<Value>>,
    /// The place to store the closed upvalue.
    closed: Option<Value>,
}

impl Upvalue {
    /// Create a new open upvalue.
    pub(crate) fn open(next: Option<NonNull<Upvalue>>, pointer: NonNull<Option<Value>>) -> Self {
        Self {
            next,
            pointer,
            closed: None,
        }
    }

    fn is_closed(&self) -> bool {
        self.closed.is_none()
    }
}

pub(crate) struct Closure {
    function: Function,
    upvalues: NonNull<[NonNull<Upvalue>]>,
}

impl Closure {
    /// Create a closure that does not capture any upvalues.
    pub(crate) fn free(function: Function) -> Self {
        assert_eq!(function.upvalues, 0);

        Self {
            function,
            upvalues: LEAKING_ALLOCATOR.alloc_empty_array(),
        }
    }

    pub(crate) fn capturing(function: Function, upvalues: NonNull<[NonNull<Upvalue>]>) -> Self {
        Self { function, upvalues }
    }

    pub(crate) fn function(&self) -> &Function {
        &self.function
    }

    pub(crate) fn upvalues(&self) -> NonNull<[NonNull<Upvalue>]> {
        self.upvalues
    }
}

#[derive(Clone)]
pub(crate) enum Value {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Function(Function),
    Closure(NonNull<Closure>),
    Return(Continuation),
    Upvalue(NonNull<Upvalue>),
}

impl Value {
    pub(crate) fn display(&self) -> String {
        match self {
            Value::Nil => "<nil>".to_string(),
            Value::Boolean(b) => format!("<{}>", b),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Function(Function { name, .. }) => format!("<function {}>", name),
            // TODO: This is not safe...
            Value::Closure(closure) => unsafe {
                format!("<closure {}>", closure.as_ref().function.name)
            },
            Value::Return(continuation) => format!("<return {}>", continuation.display()),
            // TODO: This is not safe...
            Value::Upvalue(upvalue) => unsafe {
                format!(
                    "<upvalue {}>",
                    if upvalue.as_ref().is_closed() {
                        "closed"
                    } else {
                        "open"
                    }
                )
            },
        }
    }
}

impl From<Constant> for Value {
    fn from(constant: Constant) -> Self {
        match constant {
            Constant::Number(n) => Value::Number(n),
            Constant::String(s) => Value::String(s),
            Constant::Function(f) => Value::Function(f.into()),
        }
    }
}
