use anyhow::Result;
use forsm::{build, next};

fn main() -> Result<()> {
    let instance = build("Hello, world!")?;
    let result = next(&instance).unwrap();
    println!("{}", result);
    Ok(())
}
