use super::compiler::Compiler;
use anyhow::Result;
use parity_wasm::elements::{
    BlockType,
    Instruction::{self, *},
    Instructions, ValueType,
};

pub struct Generator {
    compiler: Compiler,
    push: u32,
    pop: u32,
    cp: i32,
    last_word_address: i32,
}

impl Generator {
    pub fn define_parse(self, value: Vec<u8>) -> Self {
        let compiler = self.compiler;

        let buf_start = 0x100;
        let buf_end = buf_start + value.len() as i32;
        let compiler = compiler.add_data(buf_start, value);

        let (compiler, word_start) = compiler.add_global(|g| {
            g.mutable()
                .value_type()
                .i32()
                .init_expr(I32Const(buf_start))
        });
        let (compiler, buffer_head) = compiler.add_global(|g| {
            g.mutable()
                .value_type()
                .i32()
                .init_expr(I32Const(buf_start))
        });
        let (compiler, buffer_tail) =
            compiler.add_global(|g| g.mutable().value_type().i32().init_expr(I32Const(buf_end)));

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
        let compiler = compiler.add_export("parse", |e| e.func(parse));

        Self { compiler, ..self }
    }

    pub fn define_stack(self) -> Self {
        let compiler = self.compiler;
        let (compiler, stack) =
            compiler.add_global(|g| g.mutable().value_type().i32().init_expr(I32Const(0)));

        #[rustfmt::skip]
        let push_instructions = vec![
            // increment stack pointer
            GetGlobal(stack),
            I32Const(4),
            I32Add,
            SetGlobal(stack),

            // write data
            GetGlobal(stack),
            GetLocal(0),
            I32Store(2, 0),
            End
        ];
        let (compiler, push) = compiler.add_func(vec![ValueType::I32], vec![], |f| {
            f.with_instructions(Instructions::new(push_instructions))
        });

        #[rustfmt::skip]
        let pop_instructions = vec![
            // read data
            GetGlobal(stack),
            I32Load(2, 0),

            // decrement stack pointer
            GetGlobal(stack),
            I32Const(4),
            I32Sub,
            SetGlobal(stack),
            End
        ];
        let (compiler, pop) = compiler.add_func(vec![], vec![ValueType::I32], |f| {
            f.with_instructions(Instructions::new(pop_instructions))
        });

        let compiler = compiler
            .add_export("push", |e| e.func(push))
            .add_export("pop", |e| e.func(pop));
        Self {
            compiler,
            push,
            pop,
            ..self
        }
    }

    pub fn define_math(self) -> Self {
        // let compiler = self.compiler;
        let push = self.push;
        let pop = self.pop;
        let math_op = |op| vec![Call(pop), Call(pop), op, Call(push), End];
        self.define_native_word("+", math_op(I32Add))
            .define_native_word("-", math_op(I32Sub))
            .define_native_word("*", math_op(I32Mul))
            .define_native_word("/", math_op(I32DivS))
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        self.compiler.compile()
    }

    fn define_native_word(self, name: &str, instructions: Vec<Instruction>) -> Self {
        let compiler = self.compiler;

        let (compiler, func) = compiler.add_func(vec![], vec![], |f| {
            f.with_instructions(Instructions::new(instructions))
        });
        let (compiler, index) = compiler.add_table_entry(func);

        // for testing purposes
        let compiler = compiler.add_export(name, |e| e.func(func));

        Self { compiler, ..self }.define_word(name, index, &[])
    }

    fn define_word(self, name: &str, code: u32, parameter: &[u8]) -> Self {
        let old_last_word_address = self.last_word_address;
        let last_word_address = self.cp;

        let mut data = Vec::with_capacity(1 + name.len() + 4 + 4 + parameter.len());
        data.push(name.len() as u8);
        data.extend_from_slice(name.as_bytes());
        data.extend_from_slice(&old_last_word_address.to_le_bytes());
        data.extend_from_slice(&code.to_le_bytes());
        data.extend_from_slice(parameter);

        let cp = self.cp + data.len() as i32;
        let compiler = self.compiler.add_data(self.cp, data);
        Self {
            compiler,
            cp,
            last_word_address,
            ..self
        }
    }
}
impl Default for Generator {
    fn default() -> Self {
        let compiler: Compiler = Default::default();
        Self {
            compiler: compiler.add_memory(),
            push: 0,
            pop: 0,
            cp: 0x1000,
            last_word_address: 0,
        }
    }
}
