use anyhow::Result;
use forsm::compile_interpreter;
use wasmer::{Instance, Module, Store};
use wasmer_wasi::WasiStateBuilder;

fn main() -> Result<()> {
    let binary = compile_interpreter()?;
    let mut wasi_env = WasiStateBuilder::default()
        .preopen_dir("./src/prelude")?
        .finalize()?;
    let store = Store::default();
    let module = Module::from_binary(&store, &binary)?;
    let instance = Instance::new(&module, &wasi_env.import_object(&module)?)?;
    instance.exports.get_function("_start")?.call(&[])?;
    Ok(())
}
