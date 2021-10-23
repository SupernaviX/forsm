use std::{
    collections::HashMap,
    fs, str,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use wasmer::{imports, Function, ImportObject, LazyInit, Memory, Store, WasmerEnv};

use super::Runtime;

type Block = Box<[u8; 1024]>;
pub struct InterpreterRuntime {
    runtime: Runtime,
    blocks: Arc<Mutex<HashMap<i32, Block>>>,
    stdout: Arc<Mutex<Vec<u8>>>,
}

#[derive(WasmerEnv, Clone)]
struct ForthEnv {
    blocks: Arc<Mutex<HashMap<i32, Block>>>,
    stdout: Arc<Mutex<Vec<u8>>>,
    #[wasmer(export(name = "memory"))]
    memory: LazyInit<Memory>,
}
impl ForthEnv {
    pub fn read_block(&self, block: i32, address: i32) {
        let mut blocks = self.blocks.lock().unwrap();
        let block = blocks
            .entry(block)
            .or_insert_with(|| Box::new([b' '; 1024]));
        self.write_bytes(address, block.as_ref());
    }
    pub fn emit(&self, char: i32) {
        self.stdout.lock().unwrap().push(char as u8);
    }
    pub fn type_(&self, start: i32, len: i32) {
        let bytes = self.read_bytes(start, len);
        self.stdout.lock().unwrap().extend(bytes);
    }

    fn read_bytes(&self, start: i32, len: i32) -> Vec<u8> {
        let start = start as usize;
        let len = len as usize;
        let memory = self.memory_ref().unwrap();

        unsafe {
            let ptr = memory.data_ptr().add(start);
            let slice = std::slice::from_raw_parts(ptr, len);
            slice.to_vec()
        }
    }
    fn write_bytes(&self, start: i32, bytes: &[u8]) {
        let start = start as usize;
        let len = bytes.len();
        let memory = self.memory_ref().unwrap();

        unsafe {
            let ptr = memory.data_ptr().add(start);
            let slice = std::slice::from_raw_parts_mut(ptr, len);
            slice.copy_from_slice(bytes);
        }
    }
}

impl InterpreterRuntime {
    pub fn new(binary: &[u8]) -> Result<Self> {
        let blocks = Arc::new(Mutex::new(HashMap::new()));
        let stdout = Arc::new(Mutex::new(vec![]));
        let env = ForthEnv {
            blocks: Arc::clone(&blocks),
            stdout: Arc::clone(&stdout),
            memory: Default::default(),
        };
        let runtime = Runtime::new(binary, |store| {
            InterpreterRuntime::build_imports(store, env)
        })?;
        Ok(Self {
            blocks,
            stdout,
            runtime,
        })
    }

    pub fn run_file(&self, filename: &str) -> Result<String> {
        let file = fs::read_to_string(filename)?;
        let mut all_output = vec![];
        for line in file.lines() {
            all_output.push(self.interpret(line)?);
        }
        Ok(all_output.join(""))
    }

    pub fn interpret(&self, input: &str) -> Result<String> {
        self.write_input(input)?;
        self.execute("INTERPRET")?;

        // assert no errors
        self.execute("ERROR@")?;
        let error = self.pop()?;
        if error != 0 {
            return Err(anyhow!("Interpretation threw {}", error));
        }
        self.read_output()
    }

    pub fn write_input(&self, input: &str) -> Result<()> {
        assert!(input.len() <= 1024);
        // store the input in block 1
        {
            let mut blocks = self.blocks.lock().unwrap();
            let block = blocks.entry(1).or_insert_with(|| Box::new([b' '; 1024]));
            block[0..input.len()].copy_from_slice(input.as_bytes());
            block[input.len()..1024].iter_mut().for_each(|b| *b = b' ');
        }
        // now load it!
        self.push(1)?;
        self.execute("(LOAD)")
    }

    pub fn read_output(&self) -> Result<String> {
        let mut stdout = self.stdout.lock().unwrap();
        let result = str::from_utf8(&stdout)?.to_owned();
        stdout.clear();
        Ok(result)
    }

    pub fn push(&self, value: i32) -> Result<()> {
        self.runtime.push(value)
    }

    pub fn pop(&self) -> Result<i32> {
        self.runtime.pop()
    }

    pub fn push_string(&self, start: i32, string: &str) -> Result<()> {
        self.set_string(start, string)?;
        self.push(start)?;
        self.push(string.len() as i32)?;
        Ok(())
    }

    pub fn execute(&self, word: &str) -> Result<()> {
        self.runtime.execute(word)
    }

    fn set_string(&self, start: i32, string: &str) -> Result<()> {
        let start = start as usize;
        let end = start + string.len();

        let view = &self.runtime.memory()?[start..end];
        for (cell, value) in view.iter().zip(string.as_bytes()) {
            cell.set(*value);
        }
        Ok(())
    }

    fn build_imports(store: &Store, env: ForthEnv) -> ImportObject {
        let read_block = Function::new_native_with_env(store, env.clone(), ForthEnv::read_block);
        let emit = Function::new_native_with_env(store, env.clone(), ForthEnv::emit);
        let type_ = Function::new_native_with_env(store, env, ForthEnv::type_);
        imports! {
            "IO" => {
                "READ-BLOCK" => read_block,
                "EMIT" => emit,
                "TYPE" => type_,
            }
        }
    }
}
