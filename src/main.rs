use anyhow::Result;
use forsm::build_interpreter;

fn main() -> Result<()> {
    let interpreter = build_interpreter()?;
    interpreter.write_input("Hello, world!")?;
    interpreter.push(' ' as i32)?;
    interpreter.execute("PARSE-NAME")?;
    let result = interpreter.pop_string()?;
    println!("{}", result);
    Ok(())
}
