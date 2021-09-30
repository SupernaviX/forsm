use anyhow::Result;
use parity_wasm::{
    builder::{
        self, ExportBuilder, FuncBodyBuilder, FunctionBuilder, GlobalBuilder, ModuleBuilder,
    },
    elements::{Instruction::I32Const, ValueType},
    serialize,
};

pub struct Compiler {
    builder: ModuleBuilder,
    globals: u32,
    functions: u32,
}
impl Compiler {
    pub fn add_memory(self) -> Self {
        let builder = self
            .builder
            .memory()
            .build()
            .export()
            .field("memory")
            .internal()
            .memory(0)
            .build();
        Self { builder, ..self }
    }

    pub fn add_data(self, offset: i32, value: Vec<u8>) -> Self {
        let builder = self
            .builder
            .data()
            .offset(I32Const(offset))
            .value(value)
            .build();
        Self { builder, ..self }
    }

    pub fn add_global<T>(self, define: T) -> (Self, u32)
    where
        T: FnOnce(GlobalBuilder<ModuleBuilder>) -> GlobalBuilder<ModuleBuilder>,
    {
        let builder = define(self.builder.global()).build();
        let index = self.globals;
        let result = Self {
            builder,
            globals: self.globals + 1,
            ..self
        };
        (result, index)
    }

    pub fn add_func<T>(
        self,
        params: Vec<ValueType>,
        results: Vec<ValueType>,
        body: T,
    ) -> (Self, u32)
    where
        T: FnOnce(
            FuncBodyBuilder<FunctionBuilder<ModuleBuilder>>,
        ) -> FuncBodyBuilder<FunctionBuilder<ModuleBuilder>>,
    {
        let body_builder = self
            .builder
            .function()
            .signature()
            .with_params(params)
            .with_results(results)
            .build()
            .body();
        let builder = body(body_builder).build().build();
        let index = self.functions;
        let result = Self {
            builder,
            functions: self.functions + 1,
            ..self
        };
        (result, index)
    }

    pub fn add_export<T>(self, define: T) -> Self
    where
        T: FnOnce(ExportBuilder<ModuleBuilder>) -> ExportBuilder<ModuleBuilder>,
    {
        let builder = define(self.builder.export()).build();
        Self { builder, ..self }
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        let binary = serialize(self.builder.build())?;
        Ok(binary)
    }
}

impl Default for Compiler {
    fn default() -> Self {
        let builder = builder::module();
        Self {
            builder,
            globals: 0,
            functions: 0,
        }
    }
}
