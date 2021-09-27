use anyhow::Result;
use forsm::execute;

fn main() -> Result<()> {
    let result = execute(1337)?;
    println!("Hello, world! {:?}", result);
    Ok(())
}
