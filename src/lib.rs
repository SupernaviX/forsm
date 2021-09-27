use anyhow::{anyhow, Result};
use parity_wasm::{
    builder,
    elements::{Instruction, Instructions},
    serialize,
};
use wasmer::{imports, Instance, Module, Store, Value};

pub fn execute() -> Result<i32> {
    let binary = generate_code()?;
    let instance = instantiate(&binary)?;
    let test = instance.exports.get_function("test")?;
    let result = test.call(&[])?;
    match *result {
        [Value::I32(n)] => Ok(n),
        _ => Err(anyhow!("Unexpected output {:?}", result)),
    }
}

fn generate_code() -> Result<Vec<u8>> {
    #[rustfmt::skip]
    let module = builder::module()
        .function()
            .signature()
                .result().i32()
                .build()
            .body()
                .with_instructions(Instructions::new(
                    vec![Instruction::I32Const(42), Instruction::End]
                ))
                .build()
            .build()
        .export()
            .field("test")
            .internal().func(0)
            .build()
    .build();
    Ok(serialize(module)?)
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
        let result = execute().unwrap();
        assert_eq!(result, 42);
    }
}
