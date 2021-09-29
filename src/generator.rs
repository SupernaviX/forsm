use anyhow::Result;
use parity_wasm::{
    builder::{self, ModuleBuilder},
    elements::{BlockType, Instruction::*, Instructions, ValueType},
    serialize,
};

pub struct Generator {
    builder: ModuleBuilder,
}
impl Generator {
    pub fn add_memory(self) -> Self {
        #[rustfmt::skip]
        let builder = self.builder
            .memory().build()
            .export()
                .field("memory")
                .internal().memory(0)
            .build();
        Self { builder }
    }
    pub fn add_parse(self, value: Vec<u8>) -> Self {
        const WORD_START: u32 = 0;
        const BUFFER_HEAD: u32 = 1;
        const BUFFER_TAIL: u32 = 2;

        const CHAR: u32 = 0;
        #[rustfmt::skip]
        let builder = self.builder
            .global().mutable().value_type().i32().init_expr(I32Const(0)).build()
            .global().mutable().value_type().i32().init_expr(I32Const(0)).build()
            .global().mutable().value_type().i32().init_expr(I32Const(value.len() as i32)).build();
        let builder = builder.data().offset(I32Const(0)).value(value).build();
        #[rustfmt::skip]
        let builder = builder
            .function()
                .signature()
                .with_param(ValueType::I32)
                .with_results(vec![ValueType::I32, ValueType::I32])
                .build()
            .body()
                .with_instructions(Instructions::new(vec![
                    // Ignore leading chars-to-ignore
                    Block(BlockType::NoResult),
                    Loop(BlockType::NoResult),
                        // If we done we done
                        GetGlobal(BUFFER_HEAD),
                        GetGlobal(BUFFER_TAIL),
                        I32Eq,
                        BrIf(1),

                        // If we see something besides the delimiter we done
                        GetGlobal(BUFFER_HEAD),
                        I32Load8U(0, 0),
                        GetLocal(CHAR),
                        I32Ne,
                        BrIf(1),

                        // ++i
                        GetGlobal(BUFFER_HEAD),
                        I32Const(1),
                        I32Add,
                        SetGlobal(BUFFER_HEAD),
                        Br(0),
                    End,
                    End,

                    // This is the start of the word
                    GetGlobal(BUFFER_HEAD),
                    SetGlobal(WORD_START),

                    // Keep going until we reach the end of the buffer or char-to-ignore
                    Block(BlockType::NoResult),
                    Loop(BlockType::NoResult),
                        // If we done we done
                        GetGlobal(BUFFER_HEAD),
                        GetGlobal(BUFFER_TAIL),
                        I32Eq,
                        BrIf(1),

                        // If we see char-to-ignore we done
                        GetGlobal(BUFFER_HEAD),
                        I32Load8U(0, 0),
                        GetLocal(CHAR),
                        I32Eq,
                        BrIf(1),

                        // ++i
                        GetGlobal(BUFFER_HEAD),
                        I32Const(1),
                        I32Add,
                        SetGlobal(BUFFER_HEAD),
                        Br(0),
                    End,
                    End,

                    // Return start + length
                    GetGlobal(WORD_START),
                    GetGlobal(BUFFER_HEAD),
                    GetGlobal(WORD_START),
                    I32Sub,
                    End
                ]))
                .build()
            .build();
        let builder = builder.export().field("parse").internal().func(0).build();
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
