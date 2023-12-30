#[derive(Default, Clone)]
pub struct Runtime {
    pub store: Rc<RefCell<Store>>,
    pub stack: Vec<Value>,
    pub call_stack: Vec<Frame>,
}

impl Runtime {
    pub fn from_file(file: &str, imports: Option<Vec<Box<dyn Importer>>>) -> Result<Self> {
        let store = Store::from_file(file, imports)?;
        Self.instantiate(Rc::new(RefCell::new(store)))
    }

    pub fn from_reader(
        reader: &mut impl Read,
        imports: Option<Vec<Box<dyn Importer>>>,
    ) -> Result<Self> {
        let store = Store::from_reader(reader, imports)?;
        Self.instantiate(Rc::new(RefCell::new(store)))
    }

    pub fn from_bytes<T: AsRef<[u8]>>(
        b: T,
        imports: Option<Vec<Box<dyn Importer>>>,
    ) -> Result<Self> {
        let store = Store::from_bytes(b, imports)?;
        Self.instantiate(Rc::new(RefCell::new(store)))
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
}