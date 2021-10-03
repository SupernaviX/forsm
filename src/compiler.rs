use anyhow::Result;
use parity_wasm::{
    builder::{self, ExportBuilder, ExportInternalBuilder, GlobalBuilder, ModuleBuilder},
    elements::{
        Instruction::{self, I32Const},
        Instructions, Local, ValueType,
    },
    serialize,
};

pub struct Compiler {
    builder: Option<ModuleBuilder>,
    globals: u32,
    functions: u32,
    table_entries: Vec<u32>,
}
impl Compiler {
    pub fn add_memory(&mut self) {
        self.update(|builder| {
            builder
                .memory()
                .build()
                .export()
                .field("memory")
                .internal()
                .memory(0)
                .build()
        });
    }

    pub fn add_data(&mut self, offset: i32, value: Vec<u8>) {
        self.update(|builder| builder.data().offset(I32Const(offset)).value(value).build());
    }

    pub fn add_table_entry(&mut self, func: u32) -> u32 {
        let index = self.table_entries.len() as u32;
        self.table_entries.push(func);
        index
    }

    pub fn add_global<T>(&mut self, define: T) -> u32
    where
        T: FnOnce(GlobalBuilder<ModuleBuilder>) -> GlobalBuilder<ModuleBuilder>,
    {
        self.update(|b| define(b.global()).build());
        let index = self.globals;
        self.globals += 1;
        index
    }

    pub fn add_func(
        &mut self,
        params: Vec<ValueType>,
        results: Vec<ValueType>,
        locals: Vec<ValueType>,
        instructions: Vec<Instruction>,
    ) -> u32 {
        self.update(|builder| {
            builder
                .function()
                .signature()
                .with_params(params)
                .with_results(results)
                .build()
                .body()
                .with_locals(locals.iter().map(|t| Local::new(1, *t)).collect())
                .with_instructions(Instructions::new(instructions))
                .build()
                .build()
        });
        let index = self.functions;
        self.functions += 1;
        index
    }

    pub fn add_export<T>(&mut self, field: &str, define: T)
    where
        T: FnOnce(
            ExportInternalBuilder<ExportBuilder<ModuleBuilder>>,
        ) -> ExportBuilder<ModuleBuilder>,
    {
        self.update(|b| define(b.export().field(field).internal()).build());
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        let builder = self
            .builder
            .unwrap()
            .table()
            .with_min(self.table_entries.len() as u32)
            .with_element(0, self.table_entries)
            .build();
        let module = builder.build();
        let binary = serialize(module)?;
        Ok(binary)
    }

    fn update<T>(&mut self, func: T)
    where
        T: FnOnce(ModuleBuilder) -> ModuleBuilder,
    {
        self.builder = self.builder.take().map(func);
    }
}

impl Default for Compiler {
    fn default() -> Self {
        let builder = Some(builder::module());
        Self {
            builder,
            globals: 0,
            functions: 0,
            table_entries: vec![],
        }
    }
}
