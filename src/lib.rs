mod compiler;
mod generator;
use anyhow::{anyhow, Result};
use generator::Generator;
use std::{cell::Cell, str};
use wasmer::{imports, Instance, Module, Store, Value};

pub fn build_parser(input: &str) -> Result<Instance> {
    let binary = Generator::default().define_parse(input.into()).compile()?;
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

pub fn execute(instance: &Instance, word: &str) -> Result<()> {
    let word = instance.exports.get_function(word)?;
    let result = word.call(&[])?;
    match *result {
        [] => Ok(()),
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

fn instantiate(binary: &[u8]) -> Result<Instance> {
    let store = Store::default();
    let module = Module::from_binary(&store, binary)?;
    let import_object = imports! {};
    let instance = Instance::new(&module, &import_object)?;
    Ok(instance)
}

pub fn build<T>(func: T) -> Result<Instance>
where
    T: FnOnce(&mut Generator),
{
    let mut gen = Generator::default().initialize();
    func(&mut gen);
    let binary = gen.finalize().compile()?;
    instantiate(&binary)
}

#[cfg(test)]
mod tests {
    use super::{build, build_parser, execute, generator::ColonValue::*, next_token, pop, push};

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
        let instance = build(|_| {}).unwrap();

        push(&instance, 1).unwrap();
        push(&instance, 2).unwrap();
        push(&instance, 3).unwrap();

        assert_eq!(pop(&instance).unwrap(), 3);
        assert_eq!(pop(&instance).unwrap(), 2);
        assert_eq!(pop(&instance).unwrap(), 1);
    }

    #[test]
    fn should_do_math() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 3).unwrap();
        push(&instance, 4).unwrap();
        execute(&instance, "+").unwrap();

        assert_eq!(pop(&instance).unwrap(), 7);
    }

    #[test]
    fn should_do_division() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 6).unwrap();
        push(&instance, 3).unwrap();
        execute(&instance, "/").unwrap();

        assert_eq!(pop(&instance).unwrap(), 2);
    }

    #[test]
    fn should_support_colon_words() {
        let instance = build(|gen| {
            gen.define_colon_word("TEST", vec![Lit(2), Lit(3), XT("+")]);
        })
        .unwrap();
        execute(&instance, "TEST").unwrap();
        assert_eq!(pop(&instance).unwrap(), 5);
    }

    #[test]
    fn should_support_variables() {
        let instance = build(|gen| {
            gen.define_variable_word("TESTVAR", 0);
            gen.define_colon_word(
                "TEST",
                vec![Lit(1), XT("TESTVAR"), XT("!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        execute(&instance, "TEST").unwrap();
        assert_eq!(pop(&instance).unwrap(), 1);
    }

    #[test]
    fn should_dup() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 1).unwrap();
        execute(&instance, "DUP").unwrap();
        assert_eq!(pop(&instance).unwrap(), 1);
        assert_eq!(pop(&instance).unwrap(), 1);
    }

    #[test]
    fn should_swap() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 1).unwrap();
        push(&instance, 2).unwrap();
        execute(&instance, "SWAP").unwrap();
        assert_eq!(pop(&instance).unwrap(), 1);
        assert_eq!(pop(&instance).unwrap(), 2);
    }

    #[test]
    fn should_support_literals() {
        let instance = build(|gen| {
            gen.define_colon_word("THREE", vec![Lit(3)]);
        })
        .unwrap();

        execute(&instance, "THREE").unwrap();
        assert_eq!(pop(&instance).unwrap(), 3);
    }

    #[test]
    fn should_support_stack_manip() {
        let instance = build(|gen| {
            gen.define_colon_word(
                "TEST",
                vec![Lit(3), XT("DUP"), XT("DUP"), XT("+"), XT("SWAP"), XT("/")],
            );
        })
        .unwrap();
        execute(&instance, "TEST").unwrap();
        assert_eq!(pop(&instance).unwrap(), 2);
    }

    #[test]
    fn should_support_nested_colon_calls() {
        let instance = build(|gen| {
            gen.define_colon_word("SQUARE", vec![XT("DUP"), XT("*")]);
            gen.define_colon_word("TEST", vec![Lit(3), XT("SQUARE")]);
        })
        .unwrap();
        execute(&instance, "TEST").unwrap();
        assert_eq!(pop(&instance).unwrap(), 9);
    }
}
