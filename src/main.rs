use anyhow::Result;
use forsm::{build_parser, evaluate, next_token};

fn main() -> Result<()> {
    let instance = build_parser("Hello, world!")?;
    let result = next_token(&instance).unwrap();
    println!("{}", result);

    evaluate(vec!["TWO", "THREE", "+"])?;
    Ok(())
}
