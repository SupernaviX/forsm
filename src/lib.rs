mod compiler;
mod generator;
mod interpreter_bootstrap;
mod runtime;

use anyhow::Result;
use generator::Generator;
use runtime::Runtime;

pub fn build_interpreter() -> Result<Runtime> {
    build(interpreter_bootstrap::build)
}

pub fn build<T>(func: T) -> Result<Runtime>
where
    T: FnOnce(&mut Generator),
{
    let mut gen = Generator::default();
    func(&mut gen);
    let binary = gen.compile()?;
    Runtime::new(&binary)
}

#[cfg(test)]
mod tests {
    use super::{build, build_interpreter, generator::ColonValue::*};

    #[test]
    fn should_parse_string() {
        let interpreter = build_interpreter().unwrap();
        interpreter.write_input("Hello world!").unwrap();

        interpreter.push(' ' as i32).unwrap();
        interpreter.execute("PARSE-NAME").unwrap();
        assert_eq!(interpreter.pop_string().unwrap(), "Hello");

        interpreter.push(' ' as i32).unwrap();
        interpreter.execute("PARSE-NAME").unwrap();
        assert_eq!(interpreter.pop_string().unwrap(), "world!");

        interpreter.push(' ' as i32).unwrap();
        interpreter.execute("PARSE-NAME").unwrap();
        assert_eq!(interpreter.pop_string().unwrap(), "");
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
        let dup_str = interpreter.pop_string().unwrap();
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
    fn should_emit_output() {
        let interpreter = build_interpreter().unwrap();
        let output = interpreter
            .interpret("110 EMIT 105 EMIT 99 EMIT 101 EMIT")
            .unwrap();
        assert_eq!(output, "nice");
    }

    #[test]
    fn should_manipulate_stack() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(1).unwrap();
        interpreter.push(2).unwrap();
        interpreter.push(3).unwrap();

        assert_eq!(interpreter.pop().unwrap(), 3);
        assert_eq!(interpreter.pop().unwrap(), 2);
        assert_eq!(interpreter.pop().unwrap(), 1);
    }

    #[test]
    fn should_do_math() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(3).unwrap();
        interpreter.push(4).unwrap();
        interpreter.execute("+").unwrap();

        assert_eq!(interpreter.pop().unwrap(), 7);
    }

    #[test]
    fn should_do_division() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(6).unwrap();
        interpreter.push(3).unwrap();
        interpreter.execute("/").unwrap();

        assert_eq!(interpreter.pop().unwrap(), 2);
    }

    #[test]
    fn should_do_comparisons() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(2).unwrap();
        interpreter.push(1).unwrap();
        interpreter.execute(">").unwrap();
        assert_eq!(interpreter.pop().unwrap(), -1);

        interpreter.push(1).unwrap();
        interpreter.execute("<0").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 0);
    }

    #[test]
    fn should_handle_signed_div_and_mod() {
        let interpreter = build(|_| {}).unwrap();
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

                interpreter.push(divisor).unwrap();
                interpreter.push(dividend).unwrap();
                interpreter.execute("/").unwrap();
                let quotient = interpreter.pop().unwrap();

                interpreter.push(divisor).unwrap();
                interpreter.push(dividend).unwrap();
                interpreter.execute("MOD").unwrap();
                let modulo = interpreter.pop().unwrap();

                ((divisor, dividend), (quotient, modulo))
            })
            .collect();
        assert_eq!(results, test_cases);
    }

    #[test]
    fn should_support_colon_words() {
        let interpreter = build(|gen| {
            gen.define_colon_word("TEST", vec![Lit(2), Lit(3), XT("+")]);
        })
        .unwrap();
        interpreter.execute("TEST").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 5);
    }

    #[test]
    fn should_support_variables() {
        let interpreter = build(|gen| {
            gen.define_variable_word("TESTVAR", 0);
            gen.define_colon_word(
                "TEST",
                vec![Lit(1), XT("TESTVAR"), XT("!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        interpreter.execute("TEST").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 1);
    }

    #[test]
    fn should_increment_variables() {
        let interpreter = build(|gen| {
            gen.define_variable_word("TESTVAR", 6);
            gen.define_colon_word(
                "TEST",
                vec![Lit(7), XT("TESTVAR"), XT("+!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        interpreter.execute("TEST").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 13);
    }

    #[test]
    fn should_dup() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(1).unwrap();
        interpreter.execute("DUP").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 1);
        assert_eq!(interpreter.pop().unwrap(), 1);
    }

    #[test]
    fn should_swap() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(1).unwrap();
        interpreter.push(2).unwrap();
        interpreter.execute("SWAP").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 1);
        assert_eq!(interpreter.pop().unwrap(), 2);
    }

    #[test]
    fn should_rot() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(1).unwrap();
        interpreter.push(2).unwrap();
        interpreter.push(3).unwrap();
        interpreter.execute("ROT").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 1);
        assert_eq!(interpreter.pop().unwrap(), 3);
        assert_eq!(interpreter.pop().unwrap(), 2);
    }

    #[test]
    fn should_nip() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(1).unwrap();
        interpreter.push(2).unwrap();
        interpreter.push(3).unwrap();
        interpreter.execute("NIP").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 3);
        assert_eq!(interpreter.pop().unwrap(), 1);
    }

    #[test]
    fn should_tuck() {
        let interpreter = build(|_| {}).unwrap();

        interpreter.push(1).unwrap();
        interpreter.push(2).unwrap();
        interpreter.execute("TUCK").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 2);
        assert_eq!(interpreter.pop().unwrap(), 1);
        assert_eq!(interpreter.pop().unwrap(), 2);
    }

    #[test]
    fn should_support_literals() {
        let interpreter = build(|gen| {
            gen.define_colon_word("THREE", vec![Lit(3)]);
        })
        .unwrap();

        interpreter.execute("THREE").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 3);
    }

    #[test]
    fn should_support_stack_manip() {
        let interpreter = build(|gen| {
            gen.define_colon_word(
                "TEST",
                vec![Lit(3), XT("DUP"), XT("DUP"), XT("+"), XT("SWAP"), XT("/")],
            );
        })
        .unwrap();
        interpreter.execute("TEST").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 2);
    }

    #[test]
    fn should_support_nested_colon_calls() {
        let interpreter = build(|gen| {
            gen.define_colon_word("SQUARE", vec![XT("DUP"), XT("*")]);
            gen.define_colon_word("TEST", vec![Lit(3), XT("SQUARE")]);
        })
        .unwrap();
        interpreter.execute("TEST").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 9);
    }

    #[test]
    fn should_support_branching() {
        let interpreter = build(|gen| {
            #[rustfmt::skip]
            gen.define_colon_word("UPCHAR", vec![
                XT("DUP"), XT("DUP"),
                Lit(97), XT(">="), XT("SWAP"), Lit(122), XT("<="), XT("AND"),
                QBranch(12), // Lit(32) is 8 bytes, XT("-") is 4
                Lit(32), XT("-"),
            ]);
        })
        .unwrap();

        interpreter.push('a' as i32).unwrap();
        interpreter.execute("UPCHAR").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 'A' as i32);

        interpreter.push('B' as i32).unwrap();
        interpreter.execute("UPCHAR").unwrap();
        assert_eq!(interpreter.pop().unwrap(), 'B' as i32);
    }
}
