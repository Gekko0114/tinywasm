use super::{error::Error, module::*, value::Value};
use crate::{
    binary::{
        module::{Decoder, Module},
        types::{Expr, ExprValue, FuncType, Mutability},
    },
    Importer,
};
use anyhow::{bail, Context, Result};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    rc::Rc,
};

#[derive(Debug)]
pub enum Exports {
    Func(FuncInst),
    Table(TableInst),
    Memory(MemoryInst),
    Global(GlobalInst),
}

#[derive(Default)]
pub struct Store {
    pub funcs: Vec<FuncInst>,
    pub tables: Vec<TableInst>,
    pub memory: Vec<MemoryInst>,
    pub globals: Vec<GlobalInst>,
    pub imports: Option<HashMap<String, Box<dyn Importer>>>,
    pub module: ModuleInst,
    pub start: Option<u32>,
}

impl Store {
    pub fn from_file(file: &str, imports: Option<Vec<Box<dyn Importer>>>) -> Result<Self> {
        let file = fs::File::open(file)?;
        let mut decoder = Decoder::new(file);
        let module = decoder.decode()?;
        Self::new(&module, imports)
    }

    pub fn from_reader(reader: &mut impl Read, imports: Option<Vec<Box<dyn Importer>>>) -> Result<Self> {
        let mut decoder = Decoder::new(reader);
        let module = decoder.decode()?;
        Self::new(&module, imports)
    }

    pub fn from_bytes<T: AsRef<[u8]>>(b: T, imports: Option<Vec<Box<dyn Importer>>>,) -> Result<Self> {
        let buf = Cursor::new(b);
        let mut decoder = Decoder::new(buf);
        let module = decoder.decode()?;
        Self::new(&module, imports)
    }

    pub fn new(module: &Module, importers: Option<Vec<Box<dyn Importer>>>) -> Result<Self> {
        let func_type_idxs = match module.function_section {
            Some(ref functions) => functions.clone(),
            _ => vec![],
        };

        let mut funcs = vec![];
        let mut tables = vec![];
        let mut globals = vec![];
        let mut memories = vec![];

        if let Some(ref import_section) = module.import_section {
            let importers = importers
                .as_ref()
                .with_context(|| "module has import section, but not found any imported module")?;

            for import_info in import_section {
                let module_name = import_info.module.as_str();
                let field = import_info.field.as_str();

                let importers: Vec<_> = importers
                    .iter()
                    .filter(|importer| importer.name() == module_name)
                    .collect();
                if importers.is_empty() {
                    bail!("not found import module: [}", module_name);
                }
                let importer = importers.get(0).unwrap();

                match import_info.kind {
                    crate::binary::types::ImportKind::Func(typeidx) => {
                        let idx = typeidx as usize;
                        let func_type = module
                            .type_section
                            .as_ref()
                            .with_context(|| Error::NotFoundTypeSection)?
                            .get(idx)
                            .with_context(|| Error::NotFoundFuncType(idx))?;

                        let func_type = FuncType {
                            params: func_type.params.clone(),
                            results: func_type.results.clone(),
                        };

                        let func = FuncInst::External(ExternalFuncInst {
                            module: module_name.to_string(),
                            field: field.to_string(),
                            func_type,
                        });
                        funcs.push(func);
                    }
                    crate::binary::types::ImportKind::Table(_) => {
                        let table = importer
                            .resolve_table(module_name, field)?
                            .with_context(|| Error::NoImports)?;
                        tables.push(table);
                    }
                }
            }
        }
    }
}