use anyhow::Result;
use forsm::{build_parser, next_token};

fn main() -> Result<()> {
    let instance = build_parser("Hello, world!")?;
    let result = next_token(&instance).unwrap();
    println!("{}", result);
    Ok(())
}
