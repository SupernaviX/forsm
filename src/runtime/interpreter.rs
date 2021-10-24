use std::{
    fs, str,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use wasmer::{imports, Function, ImportObject, LazyInit, Memory, Store, WasmerEnv};

use super::Runtime;

pub struct InterpreterRuntime {
    stdin: Arc<Mutex<Vec<u8>>>,
    stdout: Arc<Mutex<Vec<u8>>>,
    runtime: Runtime,
}

#[derive(WasmerEnv, Clone)]
struct ForthEnv {
    stdin: Arc<Mutex<Vec<u8>>>,
    stdout: Arc<Mutex<Vec<u8>>>,
    #[wasmer(export(name = "memory"))]
    memory: LazyInit<Memory>,
}
impl ForthEnv {
    pub fn accept(&self, address: i32, max_len: i32) -> i32 {
        fn is_terminator(byte: u8) -> bool {
            byte == b'\r' || byte == b'\n'
        }
        let mut stdin = self.stdin.lock().unwrap();

        // strip leading line terminators from stdin
        let start = stdin
            .iter()
            .position(|&b| !is_terminator(b))
            .unwrap_or_else(|| stdin.len());
        if start > 0 {
            stdin.drain(0..start);
        }

        // pull the next line out of the vec and write it to the buffer
        let len = stdin
            .iter()
            .position(|&b| is_terminator(b))
            .unwrap_or_else(|| stdin.len())
            .min(max_len as usize);
        let bytes = stdin.drain(0..len);
        self.write_bytes(address, bytes.as_slice());

        len as i32
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
        let stdin = Arc::new(Mutex::new(vec![]));
        let stdout = Arc::new(Mutex::new(vec![]));
        let env = ForthEnv {
            stdin: Arc::clone(&stdin),
            stdout: Arc::clone(&stdout),
            memory: Default::default(),
        };
        let runtime = Runtime::new(binary, |store| {
            InterpreterRuntime::build_imports(store, env)
        })?;
        Ok(Self {
            stdin,
            stdout,
            runtime,
        })
    }

    pub fn run_directory(&self, dir: &str) -> Result<String> {
        let entries = fs::read_dir(dir)?;
        let mut output = vec![];
        for entry in entries {
            let path = entry?.path();
            let file = fs::read_to_string(path)?;
            output.push(self.interpret(&file)?);
        }
        Ok(output.join(""))
    }

    pub fn interpret(&self, input: &str) -> Result<String> {
        self.write_input(input)?;
        self.execute("QUIT")?;

        // assert no errors
        self.execute("ERROR@")?;
        let error = self.pop()?;
        if error != 0 {
            return Err(anyhow!("Interpretation threw {}", error));
        }
        self.read_output()
    }

    pub fn write_input(&self, input: &str) -> Result<()> {
        self.stdin.lock().unwrap().extend(input.as_bytes());
        Ok(())
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
        let accept = Function::new_native_with_env(store, env.clone(), ForthEnv::accept);
        let emit = Function::new_native_with_env(store, env.clone(), ForthEnv::emit);
        let type_ = Function::new_native_with_env(store, env, ForthEnv::type_);
        imports! {
            "IO" => {
                "ACCEPT" => accept,
                "EMIT" => emit,
                "TYPE" => type_,
            }
        }
    }
}
