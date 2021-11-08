#![cfg(test)]
use std::str;

mod interpreter;
use anyhow::{anyhow, Result};
pub use interpreter::InterpreterRuntime;
use wasmer::{ImportObject, Instance, MemoryView, Module, Store, Value};

pub struct Runtime {
    instance: Instance,
}

impl Runtime {
    pub fn new<F>(binary: &[u8], imports: F) -> Result<Self>
    where
        F: FnOnce(&Store, &Module) -> ImportObject,
    {
        let store = Store::default();
        let module = Module::from_binary(&store, binary)?;
        let instance = Instance::new(&module, &imports(&store, &module))?;
        Ok(Self { instance })
    }

    pub fn push(&self, value: i32) -> Result<()> {
        let push = self.instance.exports.get_function("push")?;
        let result = push.call(&[Value::I32(value)])?;
        match *result {
            [] => Ok(()),
            _ => Err(anyhow!("Unexpected output {:?}", result)),
        }
    }

    pub fn push_double(&self, value: i64) -> Result<()> {
        let push = self.instance.exports.get_function("push_d")?;
        let result = push.call(&[Value::I64(value)])?;
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

    pub fn pop_double(&self) -> Result<i64> {
        let pop = self.instance.exports.get_function("pop_d")?;
        let result = pop.call(&[])?;
        match *result {
            [Value::I64(val)] => Ok(val),
            _ => Err(anyhow!("Unexpected output {:?}", result)),
        }
    }

    #[cfg(test)]
    pub fn pop_string(&self) -> Result<String> {
        let len = self.pop()?;
        let start = self.pop()?;

        let start = start as usize;
        let end = start + len as usize;
        let view = &self.memory()?[start..end];
        let bytes: Vec<u8> = view.iter().map(|c| c.get()).collect();
        let string = str::from_utf8(&bytes)?;
        Ok(string.to_string())
    }

    pub fn execute(&self, word: &str) -> Result<()> {
        let word = self.instance.exports.get_function(word)?;
        let result = word.call(&[])?;
        match *result {
            [] => Ok(()),
            _ => Err(anyhow!("Unexpected output {:?}", result)),
        }
    }

    pub fn memory(&self) -> Result<MemoryView<u8>> {
        let view = self.instance.exports.get_memory("memory")?;
        Ok(view.view())
    }
}
