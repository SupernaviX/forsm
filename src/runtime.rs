use std::{cell::Cell, str};

use anyhow::{anyhow, Result};
use wasmer::{imports, Instance, Module, Store, Value};

pub struct Runtime {
    instance: Instance,
}

impl Runtime {
    pub fn new(binary: &[u8]) -> Result<Self> {
        let store = Store::default();
        let module = Module::from_binary(&store, binary)?;
        let import_object = imports! {};
        let instance = Instance::new(&module, &import_object)?;
        Ok(Self { instance })
    }

    pub fn load_input(&self, input: &str) -> Result<()> {
        // Write the parser input to the TIB
        self.execute("TIB")?;
        self.execute("@")?;
        let start = self.pop()?;
        self.set_string(start, input)?;

        // Mark that there's fresh content
        self.push(input.len() as i32)?;
        self.execute("#TIB")?;
        self.execute("!")?;
        self.push(0)?;
        self.execute(">IN")?;
        self.execute("!")?;

        Ok(())
    }

    pub fn push(&self, value: i32) -> Result<()> {
        let push = self.instance.exports.get_function("push")?;
        let result = push.call(&[Value::I32(value)])?;
        match *result {
            [] => Ok(()),
            _ => Err(anyhow!("Unexpected output {:?}", result)),
        }
    }

    pub fn pop(&self) -> Result<i32> {
        let pop = self.instance.exports.get_function("pop")?;
        let result = pop.call(&[])?;
        match *result {
            [Value::I32(val)] => Ok(val),
            _ => Err(anyhow!("Unexpected output {:?}", result)),
        }
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
        let word = self.instance.exports.get_function(word)?;
        let result = word.call(&[])?;
        match *result {
            [] => Ok(()),
            _ => Err(anyhow!("Unexpected output {:?}", result)),
        }
    }

    fn set_string(&self, start: i32, string: &str) -> Result<()> {
        let start = start as usize;
        let end = start + string.len();

        let view = &self.instance.exports.get_memory("memory")?.view()[start..end];
        for (cell, value) in view.iter().zip(string.as_bytes()) {
            cell.set(*value);
        }
        Ok(())
    }

    fn get_string(&self, start: i32, len: i32) -> Result<String> {
        let start = start as usize;
        let end = start + len as usize;

        let view = &self.instance.exports.get_memory("memory")?.view()[start..end];
        let result_bytes: Vec<u8> = view.iter().map(Cell::get).collect();
        Ok(str::from_utf8(&result_bytes)?.to_owned())
    }
}
