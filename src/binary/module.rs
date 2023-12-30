

// https://www.w3.org/TR/wasm-core-1/#modules%E2%91%A0%E2%93%AA
#[derive(Debug, Default)]
pub struct Module {
    pub magic: String,
    pub version: u32,
    pub custom_section: Option<Custom>,
    pub type_section: Option<Vec<FuncType>>,
    pub import_section: Option<Vec<Import>>,
    pub function_section: Option<Vec<u32>>,
    pub table_section: Option<Vec<Table>>,
    pub memory_section: Option<Vec<Memory>>,
    pub global_section: Option<Vec<Global>>,
    pub export_section: Option<Vec<Export>>,
    pub start_section: Option<u32>,
    pub element_section: Option<Vec<Element>>,
    pub data: Option<Vec<Data>>,
    pub code_section: Option<Vec<FunctionBody>>,
}

impl<R: io::Read> Decoder<R> {
    pub fn new(reader: R) -> Self {
        let reader = BufReader::new(reader);
        Self { reader }
    }

    fn is_end(&mut self) -> Result<bool> {
        Ok(self.reader.fill_buf().map(|b| !b.is_empty())?)
    }

    fn byte(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn bytes(&mut self, num: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; num];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn decode_to_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.as_slice().try_info()?))
    }

    fn decode_to_string(&mut self, num: usize) -> Result<String> {
        let str = String::from_utf8_lossy(self.bytes(num)?.as_slice()).to_string();
        Ok(str)
    }

    fn u32(&mut self) -> Result<u32> {
        let num = leb128::read::unsigned(&mut self.reader)?;
        let num = u32::try_from(num)?;
        Ok(num)
    }

    pub fn decode_section_header(&mut self) -> Result<(SectionID, usize)> {
        let id: SectionID = FromPrimitive::from_u8(self.byte()?).unwrap();
        let size = self.u32()? as usize;
        Ok((id, size))
    }

    pub fn decode_header(&mut self) -> Result<(String, u32)> {
        let magic = self.decode_to_string(4)?;
        if magic != "\0asm" {
            bail!("invalid binary magic")
        }
        
        let version = self.decode_to_u32()?;
        if version != 1 {
            bail!("invalid binary version")
        }
        Ok((magic, version))
    }

    pub fn decode(&mut self) -> Result<Module> {
        let (magic, version) = self.decode_header()?;
        let mut module = Module {
            magic,
            version,
            ..Module::default()
        };
        while self.is_end()? {
            let (id, size) = self.decode_section_header()?;
            let bytes = self.bytes(size)?;
            let section = decode(id, &bytes)?;
            module.add_section(section);
        }
        Ok(module)
    }
}