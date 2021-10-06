use anyhow::Result;
use forsm::{build_interpreter, execute, load_input, pop_string, push};

fn main() -> Result<()> {
    let instance = build_interpreter()?;
    load_input(&instance, "Hello, world!")?;
    push(&instance, ' ' as i32)?;
    execute(&instance, "PARSE-NAME")?;
    let result = pop_string(&instance)?;
    println!("{}", result);
    Ok(())
}
