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

pub fn push(instance: &Instance, value: i32) -> Result<()> {
    let push = instance.exports.get_function("push")?;
    let result = push.call(&[Value::I32(value)])?;
    match *result {
        [] => Ok(()),
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

pub fn pop(instance: &Instance) -> Result<i32> {
    let pop = instance.exports.get_function("pop")?;
    let result = pop.call(&[])?;
    match *result {
        [Value::I32(val)] => Ok(val),
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

pub fn add(instance: &Instance) -> Result<()> {
    let add = instance.exports.get_function("+")?;
    let result = add.call(&[])?;
    match *result {
        [] => Ok(()),
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

fn generate(input: &str) -> Result<Vec<u8>> {
    Generator::default()
        .define_memory()
        .define_stack()
        .define_math()
        .define_parse(input.into())
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
    use super::{add, build, next, pop, push};

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

    #[test]
    fn should_manipulate_stack() {
        let instance = build("Hello world!").unwrap();

        push(&instance, 1).unwrap();
        push(&instance, 2).unwrap();
        push(&instance, 3).unwrap();

        assert_eq!(pop(&instance).unwrap(), 3);
        assert_eq!(pop(&instance).unwrap(), 2);
        assert_eq!(pop(&instance).unwrap(), 1);
    }

    #[test]
    fn should_do_math() {
        let instance = build("Hello world!").unwrap();

        push(&instance, 3).unwrap();
        push(&instance, 4).unwrap();
        add(&instance).unwrap();
        assert_eq!(pop(&instance).unwrap(), 7);
    }
}
