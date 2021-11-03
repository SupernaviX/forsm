use std::{fs, str};

use anyhow::{anyhow, Result};
use wasmer_wasi::{Pipe, WasiEnv, WasiStateBuilder};

use super::Runtime;

pub struct InterpreterRuntime {
    wasi_env: WasiEnv,
    runtime: Runtime,
}

impl InterpreterRuntime {
    pub fn new(binary: &[u8]) -> Result<Self> {
        let mut wasi_env = WasiStateBuilder::default()
            .stdin(Box::new(Pipe::new()))
            .stdout(Box::new(Pipe::new()))
            .finalize()
            .unwrap();
        let runtime = Runtime::new(binary, |_, module| wasi_env.import_object(module).unwrap())?;
        Ok(Self { wasi_env, runtime })
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
        let mut wasi = self.wasi_env.state();
        let stdin = wasi.fs.stdin_mut()?.as_mut().unwrap();
        stdin.write_all(input.as_bytes())?;
        Ok(())
    }

    pub fn read_output(&self) -> Result<String> {
        let mut wasi = self.wasi_env.state();
        let stdout = wasi.fs.stdout_mut()?.as_mut().unwrap();
        let mut output = Vec::with_capacity(stdout.size() as usize);
        stdout.read_to_end(&mut output)?;
        let result = str::from_utf8(&output)?.to_owned();
        Ok(result)
    }

    pub fn push(&self, value: i32) -> Result<()> {
        self.runtime.push(value)
    }

    pub fn push_double(&self, value: i64) -> Result<()> {
        self.runtime.push_double(value)
    }

    pub fn pop(&self) -> Result<i32> {
        self.runtime.pop()
    }

    pub fn pop_double(&self) -> Result<i64> {
        self.runtime.pop_double()
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
}
