use std::{cell::Cell, fs, str};

use anyhow::{anyhow, Result};
use wasmer::imports;

use super::Runtime;

pub struct InterpreterRuntime {
    runtime: Runtime,
}

impl InterpreterRuntime {
    pub fn new(binary: &[u8]) -> Result<Self> {
        let runtime = Runtime::new(binary, |_| imports! {})?;
        Ok(Self { runtime })
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
        self.execute("EVALUATE")?;

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
        // ask the program to dump output
        self.execute("DUMP-OUTPUT-BUFFER")?;
        // and just pop off
        self.pop_string()
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

    pub fn pop_string(&self) -> Result<String> {
        let len = self.pop()?;
        let start = self.pop()?;
        self.get_string(start, len)
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

    fn get_string(&self, start: i32, len: i32) -> Result<String> {
        let start = start as usize;
        let end = start + len as usize;

        let view = &self.runtime.memory()?[start..end];
        let result_bytes: Vec<u8> = view.iter().map(Cell::get).collect();
        Ok(str::from_utf8(&result_bytes)?.to_owned())
    }
}
