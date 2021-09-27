mod generator;
use anyhow::{anyhow, Result};
use generator::Generator;
use wasmer::{imports, Instance, Module, Store, Value};

pub fn execute(output: i32) -> Result<i32> {
    let binary = generate_code(output)?;
    let instance = instantiate(&binary)?;
    let test = instance.exports.get_function("test")?;
    let result = test.call(&[])?;
    match *result {
        [Value::I32(n)] => Ok(n),
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

fn generate_code(output: i32) -> Result<Vec<u8>> {
    Generator::default().add_test_func(output).compile()
}

fn instantiate(binary: &[u8]) -> Result<Instance> {
    let store = Store::default();
    let module = Module::from_binary(&store, binary)?;
    let import_object = imports! {};
    let instance = Instance::new(&module, &import_object)?;
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::execute;

    #[test]
    fn should_run_wasm() {
        let result = execute(42).unwrap();
        assert_eq!(result, 42);
    }
}
