use super::module::{FuncInst, InternalFuncInst};
use super::op::*;
use super::store::{Exports, Store};
use super::value::{ExternalVal, Frame, Label, StackAccess, Value};
use crate::binary::instruction::*;
use crate::execution::error::Error;
use crate::execution::value::LabelKind;
use crate::{load, store, Importer};
use anyhow::{bail, Context as _, Result};
use log::{error, trace};
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct Runtime {
    pub store: Rc<RefCell<Store>>,
    pub stack: Vec<Value>,
    pub call_stack: Vec<Frame>,
}

impl Runtime {
    pub fn from_file(file: &str, imports: Option<Vec<Box<dyn Importer>>>) -> Result<Self> {
        let store = Store::from_file(file, imports)?;
        Self::instantiate(Rc::new(RefCell::new(store)))
    }

    pub fn from_reader(
        reader: &mut impl Read,
        imports: Option<Vec<Box<dyn Importer>>>,
    ) -> Result<Self> {
        let store = Store::from_reader(reader, imports)?;
        Self::instantiate(Rc::new(RefCell::new(store)))
    }

    pub fn from_bytes<T: AsRef<[u8]>>(
        b: T,
        imports: Option<Vec<Box<dyn Importer>>>,
    ) -> Result<Self> {
        let store = Store::from_bytes(b, imports)?;
        Self::instantiate(Rc::new(RefCell::new(store)))
    }

    pub fn instantiate(store: Rc<RefCell<Store>>) -> Result<Self> {
        let start = store.borrow().start;
        let mut runtime = Self {
            store,
            ..Default::default()
        };
        if let Some(idx) = start {
            let result = runtime.call_start(idx as usize, vec![])?;
            if let Some(out) = result {
                runtime.stack.push(out);
            }
        }
        Ok(runtime)
    }

    pub fn call(&mut self, name: String, args: Vec<Value>) -> Result<Option<Value>> {
        trace!("call function: {}", name);
        for arg in args {
            self.stack.push(arg);
        }

        let idx = {
            let store = self.store.borrow();
            let export_inst = store
                .module
                .exports
                .get(&name)
                .with_context(|| Error::NotFoundExportInstance(name))?;
            let external_val = &export_inst.desc;
            let ExternalVal::Func(idx) = external_val else {
                bail!("invalid export desc: {:?}", external_val);
            };
            *idx as usize
        };
        self.invoke(idx)
    }

    pub fn call_start(&mut self, idx: usize, args: Vec<Value>) -> Result<Option<Value>> {
        for arg in args {
            self.stack.push(arg);
        }
        self.invoke(idx)
    }

    pub fn exports(&mut self, name: String) -> Result<Exports> {
        let store = self.store.borrow();
        let export_inst = store
            .module
            .exports
            .get(&name)
            .with_context(|| Error::NotFoundExportInstance(name))?;

            let exports = match export_inst.desc {
                ExternalVal::Table(idx) => {
                    let table = store
                        .tables
                        .get(idx as usize)
                        .with_context(|| Error::NotFoundExportedTable(idx))?;
                    Exports::Table(Rc::clone(table))
                }
                ExternalVal::Memory(idx) => {
                    let memory = store
                        .memory
                        .get(idx as usize)
                        .with_context(|| Error::NotFoundExportedMemory(idx))?;
                    Exports::Memory(Rc::clone(memory))
                }
                ExternalVal::Global(idx) => {
                    let global = store
                        .globals
                        .get(idx as usize)
                        .with_context(|| Error::NotFoundExportedGlobal(idx))?;
                    Exports::Global(Rc::clone(global))
                }
                ExternalVal::Func(idx) => {
                    let func = store
                        .funcs
                        .get(idx as usize)
                        .with_context(|| Error::NotFoundExportedFunction(idx))?;
                    Exports::Func(func.clone())
                }
            };
            Ok(exports)
    }

    fn invoke_internal(&mut self, func: InternalFuncInst) -> Result<Option<Value>> {
        let arity = func.func_type.results.len();

        push_frame(&mut self.stack, &mut self.call_stack, &func);
        self.execute()?;
        let result = if arity > 0 {
            let value: Value = self.stack.pop1()?;
            Some(value)
        } else {
            None
        };
        Ok(result)
    }

    fn invoke(&mut self, idx: usize) -> Result<Option<Value>> {
        let func = self.get_func_by_idx(idx)?;
        let result = match func {
            FuncInst::Internal(func) => self.invoke_internal(func),
            FuncInst::External(func) => {
                let stack = &mut self.stack;
                invoke_external(Rc::clone(&self.store), stack, func)
            }
        };
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                self.stack = vec![];
                self.call_stack = vec![];
                Err(e)
            }
        }

        fn get_func_by_idx(&mut self, idx: usize) -> Result<FuncInst> {
            let store = self.store.borrow();
            let func = store
                .funcs
                .get(idx)
                .with_context(|| Error::NotFoundFunction(idx))?;
            Ok(func.close())
        }
    }


}