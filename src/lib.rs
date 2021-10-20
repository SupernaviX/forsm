mod compiler;
mod generator;
mod interpreter_bootstrap;
mod runtime;

use anyhow::Result;
use generator::Generator;
use runtime::{InterpreterRuntime, Runtime};
use wasmer::{imports, ImportObject, Store};

pub fn build_interpreter() -> Result<InterpreterRuntime> {
    let mut gen = Generator::default();
    interpreter_bootstrap::build(&mut gen);
    let binary = gen.compile()?;
    InterpreterRuntime::new(&binary)
}

pub fn build<T>(func: T) -> Result<Runtime>
where
    T: FnOnce(&mut Generator),
{
    build_with_imports(func, |_| imports! {})
}

pub fn build_with_imports<T, F>(func: T, imports: F) -> Result<Runtime>
where
    T: FnOnce(&mut Generator),
    F: FnOnce(&Store) -> ImportObject,
{
    let mut gen = Generator::default();
    func(&mut gen);
    let binary = gen.compile()?;
    Runtime::new(&binary, imports)
}

#[cfg(test)]
mod tests {
    use wasmer::{imports, Function};

    use super::{build, build_interpreter, build_with_imports, generator::ColonValue::*};

    #[test]
    fn should_parse_string() {
        let interpreter = build_interpreter().unwrap();
        interpreter.write_input("Hello world!").unwrap();

        interpreter.push(' ' as i32).unwrap();
        interpreter.execute("PARSE-NAME").unwrap();
        interpreter.execute("TYPE").unwrap();
        assert_eq!(interpreter.read_output().unwrap(), "Hello");

        interpreter.push(' ' as i32).unwrap();
        interpreter.execute("PARSE-NAME").unwrap();
        interpreter.execute("TYPE").unwrap();
        assert_eq!(interpreter.read_output().unwrap(), "world!");

        interpreter.push(' ' as i32).unwrap();
        interpreter.execute("PARSE-NAME").unwrap();
        interpreter.execute("TYPE").unwrap();
        assert_eq!(interpreter.read_output().unwrap(), "");
    }

    #[test]
    fn should_handle_string_equality() {
        let interpreter = build_interpreter().unwrap();
        let addr1 = 0x500;
        let addr2 = 0x600;

        interpreter.push_string(addr1, "Fred").unwrap();
        interpreter.push_string(addr2, "FRED").unwrap();
        interpreter.execute("STR-UPPER-EQ?").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1);

        interpreter.push_string(addr1, "Fred").unwrap();
        interpreter.push_string(addr2, "George").unwrap();
        interpreter.execute("STR-UPPER-EQ?").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 0);
    }

    #[test]
    fn should_find_words() {
        let interpreter = build_interpreter().unwrap();
        let addr1 = 0x500;

        interpreter.push_string(addr1, "dup").unwrap();
        interpreter.execute("FIND-NAME").unwrap();
        let dup_nt = interpreter.pop().unwrap();
        assert_ne!(dup_nt, 0);

        interpreter.push(dup_nt).unwrap();
        interpreter.execute("NAME>STRING").unwrap();
        interpreter.execute("TYPE").unwrap();
        let dup_str = interpreter.read_output().unwrap();
        assert_eq!(dup_str, "DUP");

        interpreter.push_string(addr1, "DOOP").unwrap();
        interpreter.execute("FIND-NAME").unwrap();
        let doop_nt = interpreter.pop().unwrap();
        assert_eq!(doop_nt, 0);
    }

    #[test]
    fn should_parse_digits() {
        let interpreter = build_interpreter().unwrap();

        interpreter.push('6' as i32).unwrap();
        interpreter.execute("?DIGIT").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1);
        assert_eq!(interpreter.pop().unwrap(), 6);

        interpreter.push('4' as i32).unwrap();
        interpreter.execute("?DIGIT").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1);
        assert_eq!(interpreter.pop().unwrap(), 4);

        interpreter.push('a' as i32).unwrap();
        interpreter.execute("?DIGIT").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 0);

        interpreter.push(16).unwrap();
        interpreter.execute("BASE").unwrap();
        interpreter.execute("!").unwrap();
        interpreter.push('a' as i32).unwrap();
        interpreter.execute("?DIGIT").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1);
        assert_eq!(interpreter.pop().unwrap(), 10);
    }

    #[test]
    fn should_parse_numbers() {
        let interpreter = build_interpreter().unwrap();
        let addr1 = 0x500;

        interpreter.push_string(addr1, "64").unwrap();
        interpreter.execute("?NUMBER").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1); // forth true
        assert_eq!(interpreter.pop().unwrap(), 64);
    }

    #[test]
    fn should_parse_negative_numbers() {
        let interpreter = build_interpreter().unwrap();
        let addr1 = 0x500;

        interpreter.push_string(addr1, "-64").unwrap();
        interpreter.execute("?NUMBER").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1); // forth true
        assert_eq!(interpreter.pop().unwrap(), -64);
    }

    #[test]
    fn should_not_parse_numbers_in_wrong_base() {
        let interpreter = build_interpreter().unwrap();
        let addr1 = 0x500;

        interpreter.push_string(addr1, "f0").unwrap();
        interpreter.execute("?NUMBER").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 0); // forth false
    }

    #[test]
    fn should_parse_hex_literals() {
        let interpreter = build_interpreter().unwrap();
        let addr1 = 0x500;

        interpreter.push(16).unwrap();
        interpreter.execute("BASE").unwrap();
        interpreter.execute("!").unwrap();
        interpreter.push_string(addr1, "f0").unwrap();
        interpreter.execute("?NUMBER").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1);
        assert_eq!(interpreter.pop().unwrap(), 0xf0);
    }

    #[test]
    fn should_evaluate() {
        let interpreter = build_interpreter().unwrap();
        let output = interpreter.interpret("2 3 +").unwrap();

        // assert expected output
        assert_eq!(output, "");
        assert_eq!(interpreter.pop().unwrap(), 5);
    }

    #[test]
    fn should_parse_while_interpreting() {
        let interpreter = build_interpreter().unwrap();
        let output = interpreter.interpret("32 PARSE-NAME ASS").unwrap();
        assert_eq!(output, "");

        interpreter.execute("TYPE").unwrap();
        assert_eq!(interpreter.read_output().unwrap(), "ASS");
    }

    #[test]
    fn should_emit_output() {
        let interpreter = build_interpreter().unwrap();
        let output = interpreter
            .interpret("110 EMIT 105 EMIT 99 EMIT 101 EMIT")
            .unwrap();
        assert_eq!(output, "nice");
    }

    #[test]
    fn should_type_output() {
        let interpreter = build_interpreter().unwrap();
        let output = interpreter.interpret("32 parse-name k3wl! type").unwrap();
        assert_eq!(output, "k3wl!");
    }

    #[test]
    fn should_manipulate_stack() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();

        assert_eq!(runtime.pop().unwrap(), 3);
        assert_eq!(runtime.pop().unwrap(), 2);
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_do_math() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(3).unwrap();
        runtime.push(4).unwrap();
        runtime.execute("+").unwrap();

        assert_eq!(runtime.pop().unwrap(), 7);
    }

    #[test]
    fn should_do_division() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(6).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("/").unwrap();

        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_do_comparisons() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(2).unwrap();
        runtime.push(1).unwrap();
        runtime.execute(">").unwrap();
        assert_eq!(runtime.pop().unwrap(), -1);

        runtime.push(1).unwrap();
        runtime.execute("<0").unwrap();
        assert_eq!(runtime.pop().unwrap(), 0);
    }

    #[test]
    fn should_handle_signed_div_and_mod() {
        let runtime = build(|_| {}).unwrap();
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

                runtime.push(divisor).unwrap();
                runtime.push(dividend).unwrap();
                runtime.execute("/").unwrap();
                let quotient = runtime.pop().unwrap();

                runtime.push(divisor).unwrap();
                runtime.push(dividend).unwrap();
                runtime.execute("MOD").unwrap();
                let modulo = runtime.pop().unwrap();

                ((divisor, dividend), (quotient, modulo))
            })
            .collect();
        assert_eq!(results, test_cases);
    }

    #[test]
    fn should_support_colon_words() {
        let runtime = build(|gen| {
            gen.define_colon_word("TEST", vec![Lit(2), Lit(3), XT("+")]);
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 5);
    }

    #[test]
    fn should_support_variables() {
        let runtime = build(|gen| {
            gen.define_variable_word("TESTVAR", 0);
            gen.define_colon_word(
                "TEST",
                vec![Lit(1), XT("TESTVAR"), XT("!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_increment_variables() {
        let runtime = build(|gen| {
            gen.define_variable_word("TESTVAR", 6);
            gen.define_colon_word(
                "TEST",
                vec![Lit(7), XT("TESTVAR"), XT("+!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 13);
    }

    #[test]
    fn should_dup() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.execute("DUP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_swap() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.execute("SWAP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_rot() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("ROT").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 3);
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_backwards_rot() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("-ROT").unwrap();
        assert_eq!(runtime.pop().unwrap(), 2);
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 3);
    }

    #[test]
    fn should_nip() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("NIP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 3);
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_tuck() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.execute("TUCK").unwrap();
        assert_eq!(runtime.pop().unwrap(), 2);
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_support_literals() {
        let runtime = build(|gen| {
            gen.define_colon_word("THREE", vec![Lit(3)]);
        })
        .unwrap();

        runtime.execute("THREE").unwrap();
        assert_eq!(runtime.pop().unwrap(), 3);
    }

    #[test]
    fn should_support_stack_manip() {
        let runtime = build(|gen| {
            gen.define_colon_word(
                "TEST",
                vec![Lit(3), XT("DUP"), XT("DUP"), XT("+"), XT("SWAP"), XT("/")],
            );
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_support_nested_colon_calls() {
        let runtime = build(|gen| {
            gen.define_colon_word("SQUARE", vec![XT("DUP"), XT("*")]);
            gen.define_colon_word("TEST", vec![Lit(3), XT("SQUARE")]);
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 9);
    }

    #[test]
    fn should_support_branching() {
        let runtime = build(|gen| {
            #[rustfmt::skip]
            gen.define_colon_word("UPCHAR", vec![
                XT("DUP"), XT("DUP"),
                Lit(97), XT(">="), XT("SWAP"), Lit(122), XT("<="), XT("AND"),
                QBranch(12), // Lit(32) is 8 bytes, XT("-") is 4
                Lit(32), XT("-"),
            ]);
        })
        .unwrap();

        runtime.push('a' as i32).unwrap();
        runtime.execute("UPCHAR").unwrap();
        assert_eq!(runtime.pop().unwrap(), 'A' as i32);

        runtime.push('B' as i32).unwrap();
        runtime.execute("UPCHAR").unwrap();
        assert_eq!(runtime.pop().unwrap(), 'B' as i32);
    }

    #[test]
    fn should_support_imports() {
        let runtime = build_with_imports(
            |gen| {
                gen.define_imported_word("test", "SEVENTEEN", 0, 2);
                gen.define_imported_word("test", "SWALLOW", 2, 0);
                gen.define_imported_word("test", "TRIM", 2, 2);
            },
            |store| {
                imports! {
                    "test" => {
                        "seventeen" => Function::new_native(store, || (10, 7)),
                        "swallow" => Function::new_native(store, |_: i32, _: i32| {}),
                        "trim" => Function::new_native(store, |a: i32, b: i32| {
                            (a + 4, b - 8)
                        }),
                    }
                }
            },
        )
        .unwrap();

        runtime.execute("SEVENTEEN").unwrap();
        runtime.execute("+").unwrap();
        assert_eq!(runtime.pop().unwrap(), 17);

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("SWALLOW").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);

        runtime.push(0).unwrap();
        runtime.push(16).unwrap();
        runtime.execute("TRIM").unwrap();
        assert_eq!(runtime.pop().unwrap(), 8);
        assert_eq!(runtime.pop().unwrap(), 4);
    }
}
