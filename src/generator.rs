use super::compiler::Compiler;
use anyhow::Result;
use parity_wasm::elements::{
    BlockType,
    Instruction::{self, *},
    Instructions, ValueType,
};
use std::collections::HashMap;

pub struct Generator {
    compiler: Compiler,
    push: u32,
    pop: u32,
    docon: u32,
    dovar: u32,
    cp: i32,
    last_word_address: i32,
    execution_tokens: HashMap<String, i32>,
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
        let push = self.push;
        let pop = self.pop;
        let math_op = |op| vec![Call(pop), Call(pop), op, Call(push)];
        self.define_native_word("+", math_op(I32Add))
            .define_native_word("-", math_op(I32Sub))
            .define_native_word("*", math_op(I32Mul))
            .define_native_word("/", math_op(I32DivS))
    }

    pub fn define_constants(self) -> Self {
        let push = self.push;
        let (result, _, docon) = self.create_execution_token(vec![
            // The value of our parameter is the value of the constant, just fetch and push it
            GetLocal(0),
            I32Load(2, 0),
            Call(push),
        ]);
        Self { docon, ..result }.define_constant_word("DOCON", docon as i32)
    }

    pub fn define_variables(self) -> Self {
        let push = self.push;
        let pop = self.pop;
        let (result, _, dovar) = self.create_execution_token(vec![
            // the address of our parameter IS the address of the variable, just push it
            GetLocal(0),
            Call(push),
        ]);
        Self { dovar, ..result }
            .define_constant_word("DOVAR", dovar as i32)
            .define_native_word("!", vec![Call(pop), Call(pop), I32Store(2, 0)])
            .define_native_word("@", vec![Call(pop), I32Load(2, 0), Call(push)])
    }

    pub fn define_interpreter(self) -> Self {
        let pop = self.pop;
        self.define_native_word(
            "EXECUTE",
            vec![
                // get the execution token from the stack
                Call(pop),
                // h4x: this function takes a parameter which it doesn't need,
                // use the slot as a local for the XT
                TeeLocal(0),
                // in this system, an execution token is the 32-bit address of a table index,
                // with the parameter data (if any) stored immediately after it.
                // Call the func with the address of the parameter data.
                I32Const(4),
                I32Add,
                GetLocal(0),
                I32Load(2, 0),
                // h4x: the first parameter of call_indirect is a type index
                // and this library doesn't expose those. BUT push is the first-defined function
                // and it has the right type signature, so 0 works for the type index
                CallIndirect(0, 0),
            ],
        )
    }

    pub fn define_constant_word(self, name: &str, value: i32) -> Self {
        let docon = self.docon;
        self.define_word(name, docon, &value.to_le_bytes())
    }

    pub fn define_variable_word(self, name: &str, initial_value: i32) -> Self {
        let dovar = self.dovar;
        self.define_word(name, dovar, &initial_value.to_le_bytes())
    }

    pub fn define_test_word(self, name: &str, words: Vec<String>) -> Self {
        let push = self.push;
        let execute_xt = *self.execution_tokens.get("EXECUTE").unwrap();

        let mut instructions = vec![
            // store the table index of EXECUTE in our free local
            I32Const(execute_xt),
            I32Load(2, 0),
            SetLocal(0),
        ];
        for word in &words {
            // We have the execution tokens of every word,
            // so call EXECUTE with them one-at-a-time
            let xt = *self.execution_tokens.get(word).unwrap();
            instructions.push(I32Const(xt));
            instructions.push(Call(push));

            instructions.push(I32Const(0)); // give execute a dummy param on the (wasm) stack
            instructions.push(GetLocal(0));
            instructions.push(CallIndirect(0, 0));
        }

        self.define_native_word(name, instructions)
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        let cp = self.cp;
        self.define_variable_word("CP", cp).compiler.compile()
    }

    fn define_native_word(self, name: &str, instructions: Vec<Instruction>) -> Self {
        let (result, func, index) = self.create_execution_token(instructions);

        // for testing purposes, export funcs so we can call them directly from rust
        let compiler = result.compiler.add_export(name, |e| e.func(func));

        Self { compiler, ..result }.define_word(name, index, &[])
    }

    fn create_execution_token(self, mut instructions: Vec<Instruction>) -> (Self, u32, u32) {
        let compiler = self.compiler;
        instructions.push(End);
        let (compiler, func) = compiler.add_func(vec![ValueType::I32], vec![], |f| {
            f.with_instructions(Instructions::new(instructions))
        });
        let (compiler, index) = compiler.add_table_entry(func);
        (Self { compiler, ..self }, func, index)
    }

    fn define_word(mut self, name: &str, code: u32, parameter: &[u8]) -> Self {
        let old_last_word_address = self.last_word_address;
        let last_word_address = self.cp;

        let mut data = Vec::with_capacity(1 + name.len() + 4 + 4 + parameter.len());
        data.push(name.len() as u8);
        data.extend_from_slice(name.as_bytes());
        data.extend_from_slice(&old_last_word_address.to_le_bytes());
        data.extend_from_slice(&code.to_le_bytes());
        data.extend_from_slice(parameter);

        // for testing purposes, store execution tokens for later
        self.execution_tokens.insert(
            name.to_owned(),
            last_word_address + 1 + name.len() as i32 + 4,
        );

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
            docon: 0,
            dovar: 0,
            cp: 0x1000,
            last_word_address: 0,
            execution_tokens: HashMap::new(),
        }
    }
}
