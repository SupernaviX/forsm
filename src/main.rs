use anyhow::Result;
use forsm::compile_interpreter;

fn main() -> Result<()> {
    let binary = compile_interpreter()?;
    std::fs::write("./bin/forsm.wasm", &binary)?;
    println!("Compiled to ./bin/forsm.wasm. Run with:");
    println!("wasmer --dir=./src ./bin/forsm.wasm");
    Ok(())
}
