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
    push_r: u32,
    pop_r: u32,
    docon: u32,
    dovar: u32,
    docol: u32,
    execute: u32,
    cp: i32,
    last_word_address: i32,
    execution_tokens: HashMap<String, i32>,
}

impl Generator {
    pub fn define_parse(mut self, value: Vec<u8>) -> Self {
        let buf_start = 0x100;
        let buf_end = buf_start + value.len() as i32;
        self.compiler.add_data(buf_start, value);

        let word_start = self.add_global(buf_start);
        let buffer_head = self.add_global(buf_start);
        let buffer_tail = self.add_global(buf_end);

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

        let parse = self
            .compiler
            .add_func(params, results, |b| b.with_instructions(instructions));
        self.compiler.add_export("parse", |e| e.func(parse));
        self
    }

    pub fn initialize(mut self) -> Self {
        self.define_stacks();
        self.define_constants();
        self.define_variables();
        self.define_interpreter();
        self.define_math();
        self
    }

    fn define_stacks(&mut self) {
        let define_stack = |compiler: &mut Compiler, stack| {
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
                End,
            ];
            let push = compiler.add_func(vec![ValueType::I32], vec![], |f| {
                f.with_instructions(Instructions::new(push_instructions))
            });

            let pop_instructions = vec![
                // read data
                GetGlobal(stack),
                I32Load(2, 0),
                // decrement stack pointer
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
                SetGlobal(stack),
                End,
            ];
            let pop = compiler.add_func(vec![], vec![ValueType::I32], |f| {
                f.with_instructions(Instructions::new(pop_instructions))
            });

            (push, pop)
        };
        // define the normal stack
        let stack = self.add_global(0x00);
        let (push, pop) = define_stack(&mut self.compiler, stack);
        self.push = push;
        self.pop = pop;
        // define the return stack
        let r_stack = self.add_global(0x80);
        let (push_r, pop_r) = define_stack(&mut self.compiler, r_stack);
        self.push_r = push_r;
        self.pop_r = pop_r;

        self.compiler.add_export("push", |e| e.func(push));
        self.compiler.add_export("pop", |e| e.func(pop));
        self.define_native_word(
            "DUP",
            vec![
                // just push the top of the stack onto itself
                GetGlobal(stack),
                I32Load(2, 0),
                Call(push),
            ],
        );
        self.define_native_word(
            "DROP",
            vec![
                // just decrement the stack pointer
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
                SetGlobal(stack),
            ],
        );
        self.define_native_word(
            "SWAP",
            vec![
                // don't bother touching the stack pointer
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
                TeeLocal(0),
                GetLocal(0),
                I32Load(2, 0),
                GetLocal(0),
                GetLocal(0),
                I32Load(2, 4),
                I32Store(2, 0),
                I32Store(2, 4),
            ],
        );
    }

    fn define_constants(&mut self) {
        let push = self.push;
        let docon = self.create_native_callable(vec![
            // The value of our parameter is the value of the constant, just fetch and push it
            GetLocal(0),
            I32Load(2, 0),
            Call(push),
        ]);
        self.docon = docon;
        self.define_constant_word("DOCON", docon as i32);
    }

    pub fn define_variables(&mut self) {
        let push = self.push;
        let pop = self.pop;
        let dovar = self.create_native_callable(vec![
            // the address of our parameter IS the address of the variable, just push it
            GetLocal(0),
            Call(push),
        ]);
        self.dovar = dovar;
        self.define_constant_word("DOVAR", dovar as i32);
        self.define_native_word("!", vec![Call(pop), Call(pop), I32Store(2, 0)]);
        self.define_native_word("@", vec![Call(pop), I32Load(2, 0), Call(push)]);
    }

    pub fn define_interpreter(&mut self) {
        let pop = self.pop;
        let push_r = self.push_r;
        let pop_r = self.pop_r;

        // Define ; as a variable, so we can use its address as a symbol inside colon definitions.
        // Interpretation semantics are undefined, so this is totally valid!
        self.define_variable_word(";", 0);
        let exit_xt = self.get_execution_token(";");

        let ip = self.add_global(exit_xt);
        let execute = self.compiler.add_func(vec![ValueType::I32], vec![], |f| {
            f.with_instructions(Instructions::new(vec![
                // The argument is an execution token (XT).
                // In this system, an execution token is the 32-bit address of a table index,
                // with the parameter data (if any) stored immediately after it.
                // Call the func with the address of the parameter data.
                GetLocal(0),
                I32Const(4),
                I32Add,
                GetLocal(0),
                I32Load(2, 0),
                // h4x: the first parameter of call_indirect is a type index
                // and this library doesn't expose those. BUT push is the first-defined function
                // and it has the right type signature, so 0 works for the type index
                CallIndirect(0, 0),
                End,
            ]))
        });

        #[rustfmt::skip]
        let docol = self.create_native_callable(vec![
            // push IP onto the return stack
            GetGlobal(ip),
            Call(push_r),
            // Set IP to the head of our parameter
            GetLocal(0),
            SetGlobal(ip),

            // Loop until we see EXIT's XT
            Block(BlockType::NoResult),
            Loop(BlockType::NoResult),
            GetGlobal(ip),  // IP is a pointer to an XT
            I32Load(2, 0),  // Deref it to get our next XT
            TeeLocal(0),    // Hold onto it for later

            // Is it EXIT? if so we're done
            I32Const(exit_xt),
            I32Eq,
            BrIf(1),

            // Otherwise, execute it
            GetLocal(0),
            Call(execute),
            // and increment the IP
            GetGlobal(ip),
            I32Const(4),
            I32Add,
            SetGlobal(ip),

            Br(0),
            End,
            End,
            // pop the original IP back in place
            Call(pop_r),
            SetGlobal(ip),
        ]);
        self.docol = docol;
        self.execute = execute;
        self.define_constant_word("DOCOL", docol as i32);
        self.define_native_word("EXECUTE", vec![Call(pop), Call(execute)]);
    }

    pub fn define_math(&mut self) {
        let push = self.push;
        let pop = self.pop;
        let math_op = |op| {
            vec![
                //swap the top of the stack before calling the real ops
                Call(pop),
                SetLocal(0),
                Call(pop),
                GetLocal(0),
                op,
                Call(push),
            ]
        };
        self.define_native_word("+", math_op(I32Add));
        self.define_native_word("-", math_op(I32Sub));
        self.define_native_word("*", math_op(I32Mul));
        self.define_native_word("/", math_op(I32DivS));
    }

    pub fn define_constant_word(&mut self, name: &str, value: i32) {
        let docon = self.docon;
        self.define_word(name, docon, &value.to_le_bytes());
    }

    pub fn define_variable_word(&mut self, name: &str, initial_value: i32) {
        let dovar = self.dovar;
        self.define_word(name, dovar, &initial_value.to_le_bytes());
    }

    pub fn define_colon_word(&mut self, name: &str, mut words: Vec<&str>) {
        let docol = self.docol;
        words.push(";");
        let xts: Vec<u8> = words
            .iter()
            .map(|w| self.get_execution_token(w))
            .flat_map(|xt| xt.to_le_bytes())
            .collect();
        self.define_word(name, docol, &xts);
    }

    pub fn finalize(mut self) -> Self {
        // Now that we're done adding things to the dictionary,
        // define CP (a var containing the next address in the dictionary)
        // Remember that CP takes up space in the dictionary too!
        let cp = self.cp
            + 1 // the byte containing this dictionary entry's length
            + "CP".len() as i32 // the word name
            + 4 // the variable's XT
            + 4; // the variable's storage space
        self.define_variable_word("CP", cp);
        self.cp = cp;

        // For testing, export every word as a function-which-EXECUTEs-that-word
        let execute = self.execute;
        for (word, xt) in self.execution_tokens.clone() {
            let func = self.compiler.add_func(vec![], vec![], |f| {
                f.with_instructions(Instructions::new(vec![I32Const(xt), Call(execute), End]))
            });
            self.compiler.add_export(&word, |e| e.func(func));
        }
        self
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        self.compiler.compile()
    }

    fn define_native_word(&mut self, name: &str, instructions: Vec<Instruction>) {
        let code = self.create_native_callable(instructions);
        self.define_word(name, code, &[]);
    }

    fn create_native_callable(&mut self, mut instructions: Vec<Instruction>) -> u32 {
        instructions.push(End);
        let func = self.compiler.add_func(vec![ValueType::I32], vec![], |f| {
            f.with_instructions(Instructions::new(instructions))
        });
        self.compiler.add_table_entry(func)
    }

    fn get_execution_token(&self, name: &str) -> i32 {
        *self.execution_tokens.get(name).unwrap()
    }

    fn define_word(&mut self, name: &str, code: u32, parameter: &[u8]) {
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
        self.compiler.add_data(self.cp, data);
        self.cp = cp;
        self.last_word_address = last_word_address;
    }

    fn add_global(&mut self, initial_value: i32) -> u32 {
        self.compiler.add_global(|g| {
            g.mutable()
                .value_type()
                .i32()
                .init_expr(I32Const(initial_value))
        })
    }
}
impl Default for Generator {
    fn default() -> Self {
        let mut compiler: Compiler = Default::default();
        compiler.add_memory();
        Self {
            compiler,
            push: 0,
            pop: 0,
            push_r: 0,
            pop_r: 0,
            docon: 0,
            dovar: 0,
            docol: 0,
            execute: 0,
            cp: 0x1000,
            last_word_address: 0,
            execution_tokens: HashMap::new(),
        }
    }
}
