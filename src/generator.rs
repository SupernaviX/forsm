use super::compiler::Compiler;
use anyhow::Result;
use parity_wasm::elements::{BlockType, Instruction::*, Instructions, ValueType};

pub struct Generator {
    compiler: Compiler,
}
impl Generator {
    pub fn add_memory(self) -> Self {
        Self {
            compiler: self.compiler.add_memory(),
            ..self
        }
    }

    pub fn add_parse(self, value: Vec<u8>) -> Self {
        let compiler = self.compiler;

        let len = value.len() as i32;
        let compiler = compiler.add_data(0, value);

        let (compiler, word_start) =
            compiler.add_global(|g| g.mutable().value_type().i32().init_expr(I32Const(0)));
        let (compiler, buffer_head) =
            compiler.add_global(|g| g.mutable().value_type().i32().init_expr(I32Const(0)));
        let (compiler, buffer_tail) =
            compiler.add_global(|g| g.mutable().value_type().i32().init_expr(I32Const(len)));

        let params = vec![ValueType::I32];
        const CHAR: u32 = 0;
        let results = vec![ValueType::I32, ValueType::I32];
        #[rustfmt::skip]
        let instructions = Instructions::new(vec![
            // Ignore leading chars-to-ignore
            Block(BlockType::NoResult),
            Loop(BlockType::NoResult),
                // If we done we done
                GetGlobal(buffer_head),
                GetGlobal(buffer_tail),
                I32Eq,
                BrIf(1),

                // If we see something besides the delimiter we done
                GetGlobal(buffer_head),
                I32Load8U(0, 0),
                GetLocal(CHAR),
                I32Ne,
                BrIf(1),

                // ++i
                GetGlobal(buffer_head),
                I32Const(1),
                I32Add,
                SetGlobal(buffer_head),
                Br(0),
            End,
            End,

            // This is the start of the word
            GetGlobal(buffer_head),
            SetGlobal(word_start),

            // Keep going until we reach the end of the buffer or char-to-ignore
            Block(BlockType::NoResult),
            Loop(BlockType::NoResult),
                // If we done we done
                GetGlobal(buffer_head),
                GetGlobal(buffer_tail),
                I32Eq,
                BrIf(1),

                // If we see char-to-ignore we done
                GetGlobal(buffer_head),
                I32Load8U(0, 0),
                GetLocal(CHAR),
                I32Eq,
                BrIf(1),

                // ++i
                GetGlobal(buffer_head),
                I32Const(1),
                I32Add,
                SetGlobal(buffer_head),
                Br(0),
            End,
            End,

            // Return start + length
            GetGlobal(word_start),
            GetGlobal(buffer_head),
            GetGlobal(word_start),
            I32Sub,
            End
        ]);

        let (compiler, parse) =
            compiler.add_func(params, results, |b| b.with_instructions(instructions));
        let compiler = compiler.add_export(|e| e.field("parse").internal().func(parse));

        Self { compiler }
    }
    pub fn compile(self) -> Result<Vec<u8>> {
        self.compiler.compile()
    }
}
impl Default for Generator {
    fn default() -> Self {
        Self {
            compiler: Default::default(),
        }
    }
}
