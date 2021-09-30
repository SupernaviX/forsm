mod compiler;
mod generator;
use anyhow::{anyhow, Result};
use generator::Generator;
use std::{cell::Cell, str};
use wasmer::{imports, Instance, Module, Store, Value};

pub fn build(input: &str) -> Result<Instance> {
    let binary = generate(input)?;
    let instance = instantiate(&binary)?;
    Ok(instance)
}

pub fn next(instance: &Instance) -> Result<String> {
    let parse = instance.exports.get_function("parse")?;
    let result = parse.call(&[Value::I32(' ' as i32)])?;
    match *result {
        [Value::I32(start), Value::I32(len)] => {
            let memory = instance.exports.get_memory("memory")?;
            let view = memory.view();
            let start = start as usize;
            let end = start + len as usize;
            let result_bytes: Vec<u8> = view[start..end].iter().map(Cell::get).collect();
            let result = str::from_utf8(&result_bytes)?.to_owned();
            Ok(result)
        }
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

fn generate(input: &str) -> Result<Vec<u8>> {
    Generator::default()
        .add_memory()
        .add_parse(input.into())
        .compile()
}

fn instantiate(binary: &[u8]) -> Result<Instance> {
    let store = Store::default();
    let module = Module::from_binary(&store, binary)?;
    let import_object = imports! {};
    let instance = Instance::new(&module, &import_object)?;
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::{build, next};

    #[test]
    fn should_parse_string() {
        let instance = build("Hello world!").unwrap();
        let tok1 = next(&instance).unwrap();
        assert_eq!(tok1, "Hello");
        let tok2 = next(&instance).unwrap();
        assert_eq!(tok2, "world!");
        let tok3 = next(&instance).unwrap();
        assert_eq!(tok3, "");
    }
}
