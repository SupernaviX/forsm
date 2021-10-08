mod compiler;
mod generator;
mod interpreter;

use anyhow::{anyhow, Result};
use generator::Generator;
use std::{cell::Cell, str};
use wasmer::{imports, Instance, Module, Store, Value};

pub fn build_interpreter() -> Result<Instance> {
    build(interpreter::build)
}

pub fn load_input(instance: &Instance, input: &str) -> Result<()> {
    // Write the parser input to the TIB
    execute(instance, "TIB")?;
    execute(instance, "@")?;
    let start = pop(instance)?;
    set_string(instance, start, input)?;

    // Mark that there's fresh content
    push(instance, input.len() as i32)?;
    execute(instance, "#TIB")?;
    execute(instance, "!")?;
    push(instance, 0)?;
    execute(instance, ">IN")?;
    execute(instance, "!")?;

    Ok(())
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

pub fn push_string(instance: &Instance, start: i32, string: &str) -> Result<()> {
    set_string(instance, start, string)?;
    push(instance, start)?;
    push(instance, string.len() as i32)?;
    Ok(())
}

fn set_string(instance: &Instance, start: i32, string: &str) -> Result<()> {
    let start = start as usize;
    let end = start + string.len();

    let view = &instance.exports.get_memory("memory")?.view()[start..end];
    for (cell, value) in view.iter().zip(string.as_bytes()) {
        cell.set(*value);
    }
    Ok(())
}

pub fn pop_string(instance: &Instance) -> Result<String> {
    let len = pop(instance)?;
    let start = pop(instance)?;
    get_string(instance, start, len)
}

fn get_string(instance: &Instance, start: i32, len: i32) -> Result<String> {
    let start = start as usize;
    let end = start + len as usize;

    let view = &instance.exports.get_memory("memory")?.view()[start..end];
    let result_bytes: Vec<u8> = view.iter().map(Cell::get).collect();
    Ok(str::from_utf8(&result_bytes)?.to_owned())
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
    let mut gen = Generator::default();
    func(&mut gen);
    let binary = gen.compile()?;
    instantiate(&binary)
}

#[cfg(test)]
mod tests {
    use super::{
        build, build_interpreter, execute, generator::ColonValue::*, load_input, pop, pop_string,
        push, push_string,
    };

    #[test]
    fn should_parse_string() {
        let instance = build_interpreter().unwrap();
        load_input(&instance, "Hello world!").unwrap();

        push(&instance, ' ' as i32).unwrap();
        execute(&instance, "PARSE-NAME").unwrap();
        assert_eq!(pop_string(&instance).unwrap(), "Hello");

        push(&instance, ' ' as i32).unwrap();
        execute(&instance, "PARSE-NAME").unwrap();
        assert_eq!(pop_string(&instance).unwrap(), "world!");

        push(&instance, ' ' as i32).unwrap();
        execute(&instance, "PARSE-NAME").unwrap();
        assert_eq!(pop_string(&instance).unwrap(), "");
    }

    #[test]
    fn should_handle_string_equality() {
        let instance = build_interpreter().unwrap();
        let addr1 = 0x500;
        let addr2 = 0x600;

        push_string(&instance, addr1, "Fred").unwrap();
        push_string(&instance, addr2, "FRED").unwrap();
        execute(&instance, "STR-UPPER-EQ?").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1);

        push_string(&instance, addr1, "Fred").unwrap();
        push_string(&instance, addr2, "George").unwrap();
        execute(&instance, "STR-UPPER-EQ?").unwrap();
        assert_eq!(pop(&instance).unwrap(), 0);
    }

    #[test]
    fn should_find_words() {
        let instance = build_interpreter().unwrap();
        let addr1 = 0x500;

        push_string(&instance, addr1, "dup").unwrap();
        execute(&instance, "FIND-NAME").unwrap();
        let dup_nt = pop(&instance).unwrap();
        assert_ne!(dup_nt, 0);

        push(&instance, dup_nt).unwrap();
        execute(&instance, "NAME>STRING").unwrap();
        let dup_str = pop_string(&instance).unwrap();
        assert_eq!(dup_str, "DUP");

        push_string(&instance, addr1, "DOOP").unwrap();
        execute(&instance, "FIND-NAME").unwrap();
        let doop_nt = pop(&instance).unwrap();
        assert_eq!(doop_nt, 0);
    }

    #[test]
    fn should_parse_digits() {
        let instance = build_interpreter().unwrap();

        push(&instance, '6' as i32).unwrap();
        execute(&instance, "?DIGIT").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1);
        assert_eq!(pop(&instance).unwrap(), 6);

        push(&instance, '4' as i32).unwrap();
        execute(&instance, "?DIGIT").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1);
        assert_eq!(pop(&instance).unwrap(), 4);

        push(&instance, 'a' as i32).unwrap();
        execute(&instance, "?DIGIT").unwrap();
        assert_eq!(pop(&instance).unwrap(), 0);

        push(&instance, 16).unwrap();
        execute(&instance, "BASE").unwrap();
        execute(&instance, "!").unwrap();
        push(&instance, 'a' as i32).unwrap();
        execute(&instance, "?DIGIT").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1);
        assert_eq!(pop(&instance).unwrap(), 10);
    }

    #[test]
    fn should_parse_numbers() {
        let instance = build_interpreter().unwrap();
        let addr1 = 0x500;

        push_string(&instance, addr1, "64").unwrap();
        execute(&instance, "?NUMBER").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1); // forth true
        assert_eq!(pop(&instance).unwrap(), 64);
    }

    #[test]
    fn should_parse_negative_numbers() {
        let instance = build_interpreter().unwrap();
        let addr1 = 0x500;

        push_string(&instance, addr1, "-64").unwrap();
        execute(&instance, "?NUMBER").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1); // forth true
        assert_eq!(pop(&instance).unwrap(), -64);
    }

    #[test]
    fn should_not_parse_numbers_in_wrong_base() {
        let instance = build_interpreter().unwrap();
        let addr1 = 0x500;

        push_string(&instance, addr1, "f0").unwrap();
        execute(&instance, "?NUMBER").unwrap();
        assert_eq!(pop(&instance).unwrap(), 0); // forth false
    }

    #[test]
    fn should_parse_hex_literals() {
        let instance = build_interpreter().unwrap();
        let addr1 = 0x500;

        push(&instance, 16).unwrap();
        execute(&instance, "BASE").unwrap();
        execute(&instance, "!").unwrap();
        push_string(&instance, addr1, "f0").unwrap();
        execute(&instance, "?NUMBER").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1);
        assert_eq!(pop(&instance).unwrap(), 0xf0);
    }

    #[test]
    fn should_interpret() {
        let instance = build_interpreter().unwrap();
        load_input(&instance, "2 3 +").unwrap();
        execute(&instance, "INTERPRET").unwrap();

        // assert no errors
        execute(&instance, "ERROR@").unwrap();
        assert_eq!(pop(&instance).unwrap(), 0);

        // assert expected output
        assert_eq!(pop(&instance).unwrap(), 5);
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
    fn should_do_comparisons() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 2).unwrap();
        push(&instance, 1).unwrap();
        execute(&instance, ">").unwrap();
        assert_eq!(pop(&instance).unwrap(), -1);

        push(&instance, 1).unwrap();
        execute(&instance, "<0").unwrap();
        assert_eq!(pop(&instance).unwrap(), 0);
    }

    #[test]
    fn should_handle_signed_div_and_mod() {
        let instance = build(|_| {}).unwrap();
        type TestCase = ((i32, i32), (i32, i32));

        let test_cases: Vec<TestCase> = vec![
            ((7, 4), (1, 3)),
            ((-7, 4), (-2, 1)),
            ((7, -4), (-2, -1)),
            ((-7, -4), (1, -3)),
        ];

        let results: Vec<TestCase> = test_cases
            .iter()
            .map(|case| {
                let ((divisor, dividend), _) = *case;

                push(&instance, divisor).unwrap();
                push(&instance, dividend).unwrap();
                execute(&instance, "/").unwrap();
                let quotient = pop(&instance).unwrap();

                push(&instance, divisor).unwrap();
                push(&instance, dividend).unwrap();
                execute(&instance, "MOD").unwrap();
                let modulo = pop(&instance).unwrap();

                ((divisor, dividend), (quotient, modulo))
            })
            .collect();
        assert_eq!(results, test_cases);
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
    fn should_increment_variables() {
        let instance = build(|gen| {
            gen.define_variable_word("TESTVAR", 6);
            gen.define_colon_word(
                "TEST",
                vec![Lit(7), XT("TESTVAR"), XT("+!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        execute(&instance, "TEST").unwrap();
        assert_eq!(pop(&instance).unwrap(), 13);
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
    fn should_rot() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 1).unwrap();
        push(&instance, 2).unwrap();
        push(&instance, 3).unwrap();
        execute(&instance, "ROT").unwrap();
        assert_eq!(pop(&instance).unwrap(), 1);
        assert_eq!(pop(&instance).unwrap(), 3);
        assert_eq!(pop(&instance).unwrap(), 2);
    }

    #[test]
    fn should_nip() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 1).unwrap();
        push(&instance, 2).unwrap();
        push(&instance, 3).unwrap();
        execute(&instance, "NIP").unwrap();
        assert_eq!(pop(&instance).unwrap(), 3);
        assert_eq!(pop(&instance).unwrap(), 1);
    }

    #[test]
    fn should_tuck() {
        let instance = build(|_| {}).unwrap();

        push(&instance, 1).unwrap();
        push(&instance, 2).unwrap();
        execute(&instance, "TUCK").unwrap();
        assert_eq!(pop(&instance).unwrap(), 2);
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

    #[test]
    fn should_support_branching() {
        let instance = build(|gen| {
            #[rustfmt::skip]
            gen.define_colon_word("UPCHAR", vec![
                XT("DUP"), XT("DUP"),
                Lit(97), XT(">="), XT("SWAP"), Lit(122), XT("<="), XT("AND"),
                QBranch(12), // Lit(32) is 8 bytes, XT("-") is 4
                Lit(32), XT("-"),
            ]);
        })
        .unwrap();

        push(&instance, 'a' as i32).unwrap();
        execute(&instance, "UPCHAR").unwrap();
        assert_eq!(pop(&instance).unwrap(), 'A' as i32);

        push(&instance, 'B' as i32).unwrap();
        execute(&instance, "UPCHAR").unwrap();
        assert_eq!(pop(&instance).unwrap(), 'B' as i32);
    }
}
