use anyhow::Result;
use parity_wasm::{
    builder::{
        self, signature, ExportBuilder, ExportInternalBuilder, GlobalBuilder, ModuleBuilder,
    },
    elements::{
        FuncBody,
        Instruction::{self, I32Const},
        Instructions, Local, ValueType,
    },
    serialize,
};

#[derive(Clone)]
enum FuncSource {
    Imported { module: String, field: String },
    Native(FuncBody),
}
struct Func {
    pub id: u32,
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
    pub source: FuncSource,
}
impl Func {
    pub fn is_import(&self) -> bool {
        matches!(
            &self.source,
            FuncSource::Imported {
                module: _,
                field: _
            }
        )
    }
    pub fn update_func_refs(&mut self, func_indices: &[u32]) {
        if let FuncSource::Native(body) = &mut self.source {
            for elem in body.code_mut().elements_mut() {
                if let Instruction::Call(id) = elem {
                    *id = func_indices[*id as usize];
                }
            }
        }
    }
    pub fn compile(&self, mut builder: ModuleBuilder) -> ModuleBuilder {
        match &self.source {
            FuncSource::Imported { module, field } => {
                let sig = signature()
                    .with_params(self.params.clone())
                    .with_results(self.results.clone())
                    .build_sig();
                let sig_index = builder.push_signature(sig);
                builder
                    .import()
                    .module(&module)
                    .field(&field)
                    .external()
                    .func(sig_index)
                    .build()
            }
            FuncSource::Native(body) => builder
                .function()
                .signature()
                .with_params(self.params.clone())
                .with_results(self.results.clone())
                .build()
                .with_body(body.clone())
                .build(),
        }
    }
}

pub struct Compiler {
    builder: Option<ModuleBuilder>,
    globals: u32,
    functions: Vec<Func>,
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

    pub fn add_imported_func(
        &mut self,
        module: String,
        field: String,
        params: Vec<ValueType>,
        results: Vec<ValueType>,
    ) -> u32 {
        let index = self.functions.len() as u32;
        self.functions.push(Func {
            id: index,
            params,
            results,
            source: FuncSource::Imported { module, field },
        });
        // this isn't the "real" function index webassembly will use because imports come first,
        // but we will fix that at compile time.
        index
    }

    pub fn add_native_func(
        &mut self,
        params: Vec<ValueType>,
        results: Vec<ValueType>,
        locals: Vec<ValueType>,
        instructions: Vec<Instruction>,
    ) -> u32 {
        let index = self.functions.len() as u32;
        let locals = locals.into_iter().map(|t| Local::new(1, t)).collect();
        let body = FuncBody::new(locals, Instructions::new(instructions));
        self.functions.push(Func {
            id: index,
            params,
            results,
            source: FuncSource::Native(body),
        });
        // this isn't the "real" function index webassembly will use because imports come first,
        // but we will fix that at compile time.
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

    pub fn compile(mut self) -> Result<Vec<u8>> {
        self.correct_function_indices();
        let mut builder = self
            .builder
            .unwrap()
            .table()
            .with_min(self.table_entries.len() as u32)
            .with_element(0, self.table_entries)
            .build();
        for def in self.functions.iter_mut() {
            builder = def.compile(builder);
        }
        let module = builder.build();
        let binary = serialize(module)?;
        Ok(binary)
    }

    fn correct_function_indices(&mut self) {
        // Webassembly gives functions indices according to the order they were defined,
        // and requires all imports to be defined before any "native" functions.
        // Define them in the "appropriate" order
        let (imports, defs): (Vec<&mut Func>, Vec<&mut Func>) =
            self.functions.iter_mut().partition(|f| f.is_import());

        let mut real_indices = vec![0; imports.len() + defs.len()];
        let mut current_index = 0;
        for import in imports {
            real_indices[import.id as usize] = current_index;
            current_index += 1;
        }
        for def in defs {
            real_indices[def.id as usize] = current_index;
            def.update_func_refs(&real_indices);
            current_index += 1;
        }

        for table_entry in self.table_entries.iter_mut() {
            *table_entry = real_indices[*table_entry as usize];
        }
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
            functions: vec![],
            table_entries: vec![],
        }
    }
}
