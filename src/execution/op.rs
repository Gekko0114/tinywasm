use super::{
    module::{ExternalFuncInst, InternalFuncInst},
    store::Store,
    value::{Frame, Label, LabelKind, StackAccess, Value},
};
use crate::{
    binary::{instruction::Instruction, types::ValueType},
    execution::error::Error,
    impl_binary_operation, impl_cvtop_operation, impl_unary_operation,
};
use anyhow::{bail, Context as _, Result};
use log::trace;
use std::{cell::RefCell, rc::Rc};

pub fn local_get(locals: &[Value], stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value = locals
        .get(idx)
        .with_context(|| Error::NotFoundLocalVariable(idx))?;
    stack.push(value.clone());
    Ok(())
}

pub fn local_set(locals: &mut Vec<Value>, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1()?;
    if locals.len() <= idx {
        for _ in 0..(idx + 1) - locals.len() {
            locals.push(0.into());
        }
    }
    locals[idx] = value;
    Ok(())
}

pub fn local_tee(locals: &mut Vec<Value>, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1()?;
    stack.push(value.clone());
    stack.push(value);
    local_set(locals, stack, idx)?;
    Ok(())
}

pub fn global_set(store: &mut Store, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1().with_context(|| Error::StackPopError)?;
    let mut global = store
        .globals
        .get(idx)
        .with_context(|| Error::NotFoundGlobalVariable(idx))?
        .borrow_mut();
    global.value = value;
    Ok(())
}

pub fn global_get(store: &mut Store, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let global = store
        .globals
        .get(idx)
        .with_context(|| Error::NotFoundGlobalVariable(idx))?;
    stack.push(global.borrow().value.clone());
    Ok(())
}

pub fn popcnt(stack: &mut impl StackAccess) -> Result<()> {
    let value = stack.pop1().with_context(|| Error::StackPopError)?;

    match value {
        Value::I32(v) => {
            stack.push(v.count_ones() as i32);
        }
        Value::I64(v) => {
            stack.push(v.count_ones() as i64);
        }
        _ => bail!(Error::UnexpectedStackValueType(value)),
    }
    Ok(())
}

