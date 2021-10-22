use anyhow::Result;
use forsm::build_interpreter;

fn main() -> Result<()> {
    let interpreter = build_interpreter()?;
    let result = interpreter.run_file("./src/prelude/compiler.fth")?;
    println!("{}", result);
    Ok(())
}
