use anyhow::Result;
use parity_wasm::{
    builder::{self, ModuleBuilder},
    elements::{Instruction, Instructions},
    serialize,
};

pub struct Generator {
    builder: ModuleBuilder,
}
impl Generator {
    pub fn add_test_func(self, output: i32) -> Generator {
        #[rustfmt::skip]
        let builder = self.builder
            .function()
                .signature()
                    .result().i32()
                    .build()
                .body()
                    .with_instructions(Instructions::new(vec![
                        Instruction::I32Const(output),
                        Instruction::End
                    ]))
                    .build()
                .build()
            .export()
                .field("test")
                .internal().func(0)
                .build();
        Self { builder }
    }
    pub fn compile(self) -> Result<Vec<u8>> {
        let binary = serialize(self.builder.build())?;
        Ok(binary)
    }
}
impl Default for Generator {
    fn default() -> Self {
        let builder = builder::module();
        Self { builder }
    }
}
