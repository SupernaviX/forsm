mod assembler;
mod bootstrapped_interpreter;
mod compiler;
mod runtime;

use anyhow::Result;
use compiler::Compiler;
use runtime::InterpreterRuntime;

pub fn build_interpreter() -> Result<InterpreterRuntime> {
    let mut compiler = Compiler::default();
    bootstrapped_interpreter::build(&mut compiler);
    let binary = compiler.compile()?;
    InterpreterRuntime::new(&binary)
}

#[cfg(test)]
mod tests {
    use super::build_interpreter;

    #[test]
    fn should_parse_string() {
        let interpreter = build_interpreter().unwrap();
        interpreter.write_input("Hello world!").unwrap();

        interpreter.execute("PARSE-NAME").unwrap();
        interpreter.execute("TYPE").unwrap();
        assert_eq!(interpreter.read_output().unwrap(), "Hello");

        interpreter.execute("PARSE-NAME").unwrap();
        interpreter.execute("TYPE").unwrap();
        assert_eq!(interpreter.read_output().unwrap(), "world!");

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
        let output = interpreter.interpret("PARSE-NAME ASS").unwrap();
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
        let output = interpreter.interpret("parse-name k3wl! type").unwrap();
        assert_eq!(output, "k3wl!");
    }
}
