use std::{
    cell::Cell,
    fs, str,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use wasmer::{imports, Function, ImportObject, LazyInit, Memory, Store, WasmerEnv};

use super::Runtime;

pub struct InterpreterRuntime {
    runtime: Runtime,
    stdout: Arc<Mutex<Vec<u8>>>,
}

#[derive(WasmerEnv, Clone)]
struct InterpreterEnv {
    stdout: Arc<Mutex<Vec<u8>>>,
    #[wasmer(export(name = "memory"))]
    memory: LazyInit<Memory>,
}
impl InterpreterEnv {
    pub fn read_bytes(&self, start: i32, len: i32) -> Result<Vec<u8>> {
        let start = start as usize;
        let end = start + len as usize;

        let view = &self.memory.get_ref().unwrap().view()[start..end];
        Ok(view.iter().map(Cell::get).collect())
    }
}

impl InterpreterRuntime {
    pub fn new(binary: &[u8]) -> Result<Self> {
        let stdout = Arc::new(Mutex::new(vec![]));
        let env = InterpreterEnv {
            stdout: Arc::clone(&stdout),
            memory: Default::default(),
        };
        let runtime = Runtime::new(binary, |store| {
            InterpreterRuntime::build_imports(store, env)
        })?;
        Ok(Self { runtime, stdout })
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
        // request N bytes of space
        self.push(input.len() as i32)?;
        self.execute("RESERVE-INPUT-BUFFER")?;
        // address to write to is now on the stack
        let start = self.pop()?;
        self.set_string(start, input)?;

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

    fn build_imports(store: &Store, env: InterpreterEnv) -> ImportObject {
        let emit =
            Function::new_native_with_env(store, env.clone(), |env: &InterpreterEnv, char: i32| {
                env.stdout.lock().unwrap().push(char as u8);
            });
        let type_ = Function::new_native_with_env(
            store,
            env,
            |env: &InterpreterEnv, start: i32, len: i32| {
                let bytes = env.read_bytes(start, len).unwrap();
                env.stdout.lock().unwrap().extend(bytes);
            },
        );
        imports! {
            "io" => {
                "emit" => emit,
                "type" => type_,
            }
        }
    }
}
