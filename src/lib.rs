mod compiler;
mod generator;
use anyhow::{anyhow, Result};
use generator::Generator;
use std::{cell::Cell, str};
use wasmer::{imports, Instance, Module, Store, Value};

pub fn build_parser(input: &str) -> Result<Instance> {
    let binary = generate_parser(input)?;
    let instance = instantiate(&binary)?;
    Ok(instance)
}

pub fn next_token(instance: &Instance) -> Result<String> {
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

fn generate_parser(input: &str) -> Result<Vec<u8>> {
    Generator::default().define_parse(input.into()).compile()
}

fn generate_test(words: Vec<String>) -> Result<Vec<u8>> {
    let mut gen = Generator::default().initialize();
    gen.define_constant_word("ONE", 1);
    gen.define_constant_word("TWO", 2);
    gen.define_constant_word("THREE", 3);
    gen.define_variable_word("TESTVAR", 0);
    gen.define_colon_word("TEST", words);
    gen.finalize().compile()
}

fn instantiate(binary: &[u8]) -> Result<Instance> {
    let store = Store::default();
    let module = Module::from_binary(&store, binary)?;
    let import_object = imports! {};
    let instance = Instance::new(&module, &import_object)?;
    Ok(instance)
}

pub fn evaluate(words: Vec<&'static str>) -> Result<Instance> {
    let mut owned_words = vec![];
    for word in words {
        owned_words.push(word.to_owned());
    }
    let binary = generate_test(owned_words)?;
    let instance = instantiate(&binary)?;
    let test_func = instance.exports.get_function("TEST")?;
    let result = test_func.call(&[])?;
    match *result {
        [] => Ok(instance),
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

#[cfg(test)]
mod tests {
    use super::{add, build_parser, evaluate, next_token, pop, push};

    #[test]
    fn should_parse_string() {
        let instance = build_parser("Hello world!").unwrap();
        let tok1 = next_token(&instance).unwrap();
        assert_eq!(tok1, "Hello");
        let tok2 = next_token(&instance).unwrap();
        assert_eq!(tok2, "world!");
        let tok3 = next_token(&instance).unwrap();
        assert_eq!(tok3, "");
    }

    #[test]
    fn should_manipulate_stack() {
        let instance = evaluate(vec![]).unwrap();

        push(&instance, 1).unwrap();
        push(&instance, 2).unwrap();
        push(&instance, 3).unwrap();

        assert_eq!(pop(&instance).unwrap(), 3);
        assert_eq!(pop(&instance).unwrap(), 2);
        assert_eq!(pop(&instance).unwrap(), 1);
    }

    #[test]
    fn should_do_math() {
        let instance = evaluate(vec![]).unwrap();

        push(&instance, 3).unwrap();
        push(&instance, 4).unwrap();
        add(&instance).unwrap();

        assert_eq!(pop(&instance).unwrap(), 7);
    }

    #[test]
    fn should_execute() {
        let instance = evaluate(vec!["TWO", "THREE", "+"]).unwrap();

        assert_eq!(pop(&instance).unwrap(), 5);
    }

    #[test]
    fn should_support_variables() {
        let instance = evaluate(vec!["ONE", "TESTVAR", "!", "TESTVAR", "@"]).unwrap();

        assert_eq!(pop(&instance).unwrap(), 1);
    }
}
