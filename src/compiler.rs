use anyhow::Result;
use parity_wasm::{
    builder::{
        self, ExportBuilder, ExportInternalBuilder, FuncBodyBuilder, FunctionBuilder,
        GlobalBuilder, ModuleBuilder,
    },
    elements::{Instruction::I32Const, ValueType},
    serialize,
};

pub struct Compiler {
    builder: ModuleBuilder,
    globals: u32,
    functions: u32,
    table_entries: Vec<u32>,
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

    pub fn add_table_entry(mut self, func: u32) -> (Self, u32) {
        let index = self.table_entries.len() as u32;
        self.table_entries.push(func);
        (self, index)
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

    pub fn add_export<T>(self, field: &str, define: T) -> Self
    where
        T: FnOnce(
            ExportInternalBuilder<ExportBuilder<ModuleBuilder>>,
        ) -> ExportBuilder<ModuleBuilder>,
    {
        let builder = define(self.builder.export().field(field).internal()).build();
        Self { builder, ..self }
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        // create the table now that we know what belongs in it
        let builder = self
            .builder
            .table()
            .with_min(self.table_entries.len() as u32)
            .with_element(0, self.table_entries)
            .build();
        let module = builder.build();
        let binary = serialize(module)?;
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
            table_entries: vec![],
        }
    }
}
