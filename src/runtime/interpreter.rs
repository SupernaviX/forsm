use std::{
    convert::TryInto,
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
    pub fn fd_read(&self, fd: i32, iovecs_addr: u32, iovecs_len: u32, res_addr: u32) -> i32 {
        assert_eq!(fd, 0);
        assert_eq!(iovecs_len, 1);
        let iovec = self.read_u32(iovecs_addr);
        let buf = self.read_u32(iovec);
        let len = self.read_u32(iovec + 4);

        let mut stdin = self.stdin.lock().unwrap();
        let bytes_read = len.min(stdin.len() as u32);

        let bytes = stdin.drain(0..bytes_read as usize);
        self.write_bytes(buf, bytes.as_slice());

        self.write_bytes(res_addr, &bytes_read.to_le_bytes());
        0
    }
    pub fn fd_write(&self, fd: i32, ciovecs_addr: u32, ciovecs_len: u32, res_addr: u32) -> i32 {
        assert_eq!(fd, 1);
        assert_eq!(ciovecs_len, 1);
        let ciovec = self.read_u32(ciovecs_addr);
        let buf = self.read_u32(ciovec);
        let len = self.read_u32(ciovec + 4);
        let bytes = self.read_bytes(buf, len);

        let mut stdout = self.stdout.lock().unwrap();
        stdout.extend(&bytes);

        self.write_bytes(res_addr, &len.to_le_bytes());
        0
    }
    fn read_u32(&self, address: u32) -> u32 {
        let bytes = self.read_bytes(address, 4);
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn read_bytes(&self, start: u32, len: u32) -> Vec<u8> {
        let start = start as usize;
        let len = len as usize;
        let memory = self.memory_ref().unwrap();

        unsafe {
            let ptr = memory.data_ptr().add(start);
            let slice = std::slice::from_raw_parts(ptr, len);
            slice.to_vec()
        }
    }
    fn write_bytes(&self, start: u32, bytes: &[u8]) {
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
        let fd_read = Function::new_native_with_env(store, env.clone(), ForthEnv::fd_read);
        let fd_write = Function::new_native_with_env(store, env.clone(), ForthEnv::fd_write);
        imports! {
            "IO" => {
                "FD-READ" => fd_read,
                "FD-WRITE" => fd_write,
            }
        }
    }
}
