use std::str;

use anyhow::{Result};
use wasmer_wasi::{Pipe, WasiEnv, WasiStateBuilder};

use super::basic::BasicRuntime;

/* A runtime that includes WASI and can run the Forth interpreter */
pub struct InterpreterRuntime {
    wasi_env: WasiEnv,
    runtime: BasicRuntime,
}

impl InterpreterRuntime {
    pub fn new(binary: &[u8]) -> Result<Self> {
        let mut wasi_env = WasiStateBuilder::default()
            .stdin(Box::new(Pipe::new()))
            .stdout(Box::new(Pipe::new()))
            .preopen_dir(".")?
            .finalize()
            .unwrap();
        let runtime = BasicRuntime::new(binary, |_, module| wasi_env.import_object(module).unwrap())?;
        Ok(Self { wasi_env, runtime })
    }

    pub fn interpret(&self, input: &str) -> Result<String> {
        self.write_input(&format!("{} STOP", input))?;
        self.execute("_start")?;
        let result = self.read_output()?;
        // get the prompt out of there
        Ok(result.lines().skip(2).collect::<Vec<&str>>().join("\n"))
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
}
