use anyhow::Result;
use forsm::execute;

fn main() -> Result<()> {
    let result = execute()?;
    println!("Hello, world! {:?}", result);
    Ok(())
}
