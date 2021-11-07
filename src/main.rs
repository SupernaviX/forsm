use anyhow::Result;
use forsm::build_interpreter;

fn main() -> Result<()> {
    let interpreter = build_interpreter()?;
    let result = interpreter.start()?;
    println!("{}", result);
    Ok(())
}
