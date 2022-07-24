use std::{
    ptr::{addr_of, addr_of_mut, NonNull},
    rc::Rc,
};

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

    pub(crate) fn sp(&self) -> usize {
        self.sp
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
    /// The head pointer of the list of the open upvalues.
    open_upvalues_head: Option<NonNull<Upvalue>>,
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
            open_upvalues_head: None,
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
    pub(crate) fn perform_return(&mut self) {
        let fp = self.stack.fp;
        let return_value = self.stack.pop().unwrap();
        let continuation = self.stack.get_local(0);

        // Drop the call frame for this function and close upvalues pointing to the inside of it.
        self.close_upvalue(fp);

        match continuation {
            Value::Return(mut continuation) => {
                // Since the return continuation's sp is outdated, we fix it here.
                // TODO: Isn't this assuming that the caller and the callee share the stack? Is this a valid assumption?
                continuation.stack.sp = self.stack.sp;
                continuation.stack.push(return_value);
                *self = continuation;
            }
            _ => todo!("The return continuation is not a continuation"),
        }
    }

    /// Get the pointer to the object held by the current function's upvalue at the index.
    fn get_upvalue_value_ptr(&self, index: u8) -> NonNull<Option<Value>> {
        let index = usize::from(index);
        // TODO: the assumption of safety is that the upvalues stored in the closure are valid,
        // and the index is in-bounds.
        unsafe {
            let closure = self.closure();
            assert!(index < closure.function.upvalues);
            let upvalues = closure.upvalues().get_unchecked_mut(index);
            upvalues.as_ref().as_ref().pointer
        }
    }

    pub(crate) fn get_upvalue(&self, index: u8) -> Value {
        // TODO: the assumption of safety is that the upvalues stored in the closure are valid,
        // and the index is in-bounds.
        unsafe {
            self.get_upvalue_value_ptr(index)
                .as_ref()
                .as_ref()
                .unwrap()
                .clone()
        }
    }

    pub(crate) fn set_upvalue(&mut self, index: u8, value: Value) {
        let mut pointer = self.get_upvalue_value_ptr(index);
        // TODO: the assumption of safety is that the upvalues stored in the closure are valid,
        // and the index is in-bounds.
        unsafe {
            *pointer.as_mut() = Some(value);
        }
    }

    /// Get or create upvalue pointing to the stack.
    ///
    /// The upvalues are sorted by the order of the pointer to the stack slot
    /// (the greatest one comes first).
    /// When an upvalue with the same stack index is found, returns it.
    /// When not found, a new upvalue is inserted into the appropriate place and returned.
    fn get_or_create_upvalue_to_stack(&mut self, index: u8) -> NonNull<Upvalue> {
        let pointer = self.stack.get_local_ptr(index);

        let mut prev = None;
        let mut current = self.open_upvalues_head;

        while let Some(current_ptr) = current {
            // SAFETY: the upvalues in the open-upvalues list are valid
            unsafe {
                let current_ref = current_ptr.as_ref();
                match current_ref.pointer.cmp(&pointer) {
                    std::cmp::Ordering::Less => break,
                    std::cmp::Ordering::Equal => return current_ptr,
                    std::cmp::Ordering::Greater => {
                        prev = Some(current_ptr);
                        current = current_ref.next;
                    }
                }
            }
        }

        let new_upvalue = LEAKING_ALLOCATOR.alloc(Upvalue::open(current, pointer));
        match prev {
            // SAFETY: the upvalues in the open-upvalues list are valid
            Some(mut prev) => unsafe {
                prev.as_mut().next = Some(new_upvalue);
            },
            None => {
                self.open_upvalues_head = Some(new_upvalue);
            }
        }
        new_upvalue
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
                let is_local = self.code(2 + 2 * idx) > 0;
                let index = self.code(2 + 2 * idx + 1);
                if is_local {
                    self.get_or_create_upvalue_to_stack(index)
                } else {
                    // TODO: assuming the upvalues are all valid.
                    unsafe {
                        // this closure must be valid.
                        let closure = self.closure.as_ref();
                        let index = usize::from(index);
                        assert!(index < closure.upvalues.len());
                        *closure.upvalues.get_unchecked_mut(index).as_ptr()
                    }
                }
            })
            .collect();
        // SAFETY: the pointer is valid.
        let upvalues = unsafe { NonNull::new_unchecked(Box::into_raw(upvalues)) };
        let closure =
            Value::Closure(LEAKING_ALLOCATOR.alloc(Closure::capturing(function, upvalues)));
        self.stack.push(closure);
        self.advance(2 + 2 * upvalues_len);
    }

    pub(crate) fn close_upvalue(&mut self, new_sp: usize) {
        self.stack.check();
        assert!(new_sp < self.stack.sp);

        for index in (new_sp..self.stack.sp).rev() {
            // SAFETY: index is a valid stack slot, and the open_upvalues_head must point to a valid upvalue.
            unsafe {
                let mut pointer = self.stack.values.get_unchecked_mut(index);
                let value = std::mem::replace(pointer.as_mut(), None).unwrap();

                if let Some(head) = self.open_upvalues_head {
                    let head = head.as_ptr();
                    match addr_of!((*head).pointer).read().cmp(&pointer) {
                        std::cmp::Ordering::Less => {}
                        std::cmp::Ordering::Equal => {
                            let pointer_to_closed = addr_of_mut!((*head).closed);
                            // write value to closed
                            assert!(
                                std::mem::replace(&mut *pointer_to_closed, Some(value)).is_none()
                            );
                            // update pointer to point to its closed
                            addr_of_mut!((*head).pointer)
                                .write(NonNull::new_unchecked(pointer_to_closed));
                            // unlink the upvalue and update head
                            let next = std::mem::replace(&mut *addr_of_mut!((*head).next), None);
                            self.open_upvalues_head = next;
                        }
                        std::cmp::Ordering::Greater => {
                            unreachable!("open_upvalues_head must point to a valid stack slot.")
                        }
                    }
                }
            }
        }

        // rewind the sp
        self.stack.sp = new_sp;

        self.stack.check();
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
    fn open(next: Option<NonNull<Upvalue>>, pointer: NonNull<Option<Value>>) -> Self {
        Self {
            next,
            pointer,
            closed: None,
        }
    }

    // fn is_closed(&self) -> bool {
    //     self.closed.is_none()
    // }
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
    // Upvalue(NonNull<Upvalue>),
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
            // Value::Upvalue(upvalue) => unsafe {
            //     format!(
            //         "<upvalue {}>",
            //         if upvalue.as_ref().is_closed() {
            //             "closed"
            //         } else {
            //             "open"
            //         }
            //     )
            // },
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
