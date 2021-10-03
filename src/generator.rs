use super::compiler::Compiler;
use anyhow::Result;
use parity_wasm::elements::{
    BlockType,
    Instruction::{self, *},
    ValueType,
};
use std::collections::HashMap;

pub enum ColonValue {
    XT(&'static str),
    Lit(i32),
    Branch(i32),
    QBranch(i32),
}

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
    pub fn define_constant_word(&mut self, name: &str, value: i32) {
        let docon = self.docon;
        self.define_word(name, docon, &value.to_le_bytes());
    }

    pub fn define_variable_word(&mut self, name: &str, initial_value: i32) {
        let dovar = self.dovar;
        self.define_word(name, dovar, &initial_value.to_le_bytes());
    }

    pub fn define_colon_word(&mut self, name: &str, values: Vec<ColonValue>) {
        let docol = self.docol;
        let lit_xt = self.get_execution_token("LIT");
        let branch_xt = self.get_execution_token("BRANCH");
        let q_branch_xt = self.get_execution_token("?BRANCH");
        let mut bytes = vec![];
        for value in values {
            match value {
                ColonValue::XT(name) => {
                    let xt = self.get_execution_token(name);
                    bytes.extend_from_slice(&xt.to_le_bytes())
                }
                ColonValue::Lit(value) => {
                    bytes.extend_from_slice(&lit_xt.to_le_bytes());
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                ColonValue::Branch(offset) => {
                    bytes.extend_from_slice(&branch_xt.to_le_bytes());
                    bytes.extend_from_slice(&offset.to_le_bytes());
                }
                ColonValue::QBranch(offset) => {
                    bytes.extend_from_slice(&q_branch_xt.to_le_bytes());
                    bytes.extend_from_slice(&offset.to_le_bytes());
                }
            }
        }
        let exit_xt = self.get_execution_token(";");
        bytes.extend_from_slice(&exit_xt.to_le_bytes());

        self.define_word(name, docol, &bytes);
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        self.finalize().compiler.compile()
    }

    fn initialize(mut self) -> Self {
        self.define_stacks();
        self.define_constants();
        self.define_variables();
        self.define_execution();
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
            let push = compiler.add_func(vec![ValueType::I32], vec![], vec![], push_instructions);

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
            let pop = compiler.add_func(vec![], vec![ValueType::I32], vec![], pop_instructions);

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
            0,
            vec![
                // just push the top of the stack onto itself
                GetGlobal(stack),
                I32Load(2, 0),
                Call(push),
            ],
        );
        self.define_native_word(
            "DROP",
            0,
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
            0,
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
        self.define_native_word(
            "OVER",
            0,
            vec![
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
                I32Load(2, 0),
                Call(push),
            ],
        );
    }

    fn define_constants(&mut self) {
        let push = self.push;
        let docon = self.create_native_callable(
            0,
            vec![
                // The value of our parameter is the value of the constant, just fetch and push it
                GetLocal(0),
                I32Load(2, 0),
                Call(push),
            ],
        );
        self.docon = docon;
        self.define_constant_word("DOCON", docon as i32);
    }

    fn define_variables(&mut self) {
        let push = self.push;
        let pop = self.pop;
        let dovar = self.create_native_callable(
            0,
            vec![
                // the address of our parameter IS the address of the variable, just push it
                GetLocal(0),
                Call(push),
            ],
        );
        self.dovar = dovar;
        self.define_constant_word("DOVAR", dovar as i32);
        self.define_native_word("!", 0, vec![Call(pop), Call(pop), I32Store(2, 0)]);
        self.define_native_word("@", 0, vec![Call(pop), I32Load(2, 0), Call(push)]);
        self.define_native_word(
            "+!",
            0,
            vec![
                Call(pop),
                TeeLocal(0),
                GetLocal(0),
                I32Load(2, 0),
                Call(pop),
                I32Add,
                I32Store(2, 0),
            ],
        );
        self.define_native_word("C!", 0, vec![Call(pop), Call(pop), I32Store8(0, 0)]);
        self.define_native_word("C@", 0, vec![Call(pop), I32Load8U(0, 0), Call(push)]);
    }

    fn define_execution(&mut self) {
        let push = self.push;
        let pop = self.pop;
        let push_r = self.push_r;
        let pop_r = self.pop_r;

        // Define ; as a variable, so we can use its address as a symbol inside colon definitions.
        // Interpretation semantics are undefined, so this is totally valid!
        self.define_variable_word(";", 0);
        let exit_xt = self.get_execution_token(";");

        let ip = self.add_global(exit_xt);
        let execute = self.compiler.add_func(
            vec![ValueType::I32],
            vec![],
            vec![],
            vec![
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
            ],
        );

        #[rustfmt::skip]
        let docol = self.create_native_callable(0, vec![
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
        self.define_native_word("EXECUTE", 0, vec![Call(pop), Call(execute)]);
        self.define_native_word(
            "LIT",
            0,
            vec![
                // The instruction pointer is pointing to LIT's XT inside of a colon definition.
                // The value after that is a literal; push it.
                GetGlobal(ip),
                I32Const(4),
                I32Add,
                TeeLocal(0),
                I32Load(2, 0),
                Call(push),
                // also increment IP appropriately
                GetLocal(0),
                SetGlobal(ip),
            ],
        );
        self.define_native_word(
            "BRANCH",
            0,
            vec![
                // The instruction pointer is pointing to BRANCH's XT inside of a colon definition.
                // The value after that is a literal; treat it as an offset to add to IP (where 0 is "next instr")
                GetGlobal(ip),
                TeeLocal(0),
                I32Load(2, 4),
                I32Const(4), // Add 4 to account for the size of the literal
                I32Add,
                GetLocal(0),
                I32Add,
                SetGlobal(ip),
            ],
        );
        self.define_native_word(
            "?BRANCH",
            0,
            vec![
                // Branch if the head of the stack is "false" (0)
                Call(pop),
                I32Eqz,
                If(BlockType::Value(ValueType::I32)),
                // Offset is based on the literal after the IP (+4 because of the literal's size)
                GetGlobal(ip),
                TeeLocal(0),
                I32Load(2, 4),
                I32Const(4),
                I32Add,
                Else, // Offset is just 4
                GetGlobal(ip),
                SetLocal(0),
                I32Const(4),
                End,
                GetLocal(0),
                I32Add,
                SetGlobal(ip),
            ],
        );
    }

    fn define_math(&mut self) {
        let push = self.push;
        let pop = self.pop;
        let get_two_args = vec![
            //swap the top of the stack before calling the real ops
            Call(pop),
            SetLocal(0),
            Call(pop),
            GetLocal(0),
        ];
        let binary_i32 = |op| {
            let mut res = get_two_args.clone();
            res.push(op);
            res.push(Call(push));
            res
        };
        let binary_bool = |op| {
            let mut res = vec![I32Const(0)];
            res.extend_from_slice(&get_two_args);
            res.push(op);
            res.push(I32Sub);
            res.push(Call(push));
            res
        };
        self.define_native_word("+", 0, binary_i32(I32Add));
        self.define_native_word("-", 0, binary_i32(I32Sub));
        self.define_native_word("*", 0, binary_i32(I32Mul));

        #[rustfmt::skip]
        self.define_native_word("/", 1, vec![
            Call(pop),
            TeeLocal(1), // store dividend for later (TODO: check for divide by 0)
            Call(pop),
            TeeLocal(0), // store divisor as well

            // To find the "real" divisor, check if the signs mismatch
            I32Xor,
            I32Const(i32::MIN), // 0x80000000 (high bit set)
            I32And,

            If(BlockType::Value(ValueType::I32)),
            // WASM has round-towards-0 semantics, forth has round-negative.
            // So make sure that |divisor| += |dividend|-1 to compensate.
            GetLocal(0),
            GetLocal(1),
            I32Sub, // divisor -= dividend means |divisor| += |dividend|

            I32Const(1),
            I32Const(-1),
            GetLocal(0),
            I32Const(i32::MIN),
            I32And,
            Select,
            I32Add, // divisor += -sign(divisor) means |divisor| -= 1

            Else, // if divisor and dividend have matching signs, just use divisor
            GetLocal(0),
            End,

            GetLocal(1),
            I32DivS,
            Call(push),
        ]);
        #[rustfmt::skip]
        self.define_native_word("MOD", 1, vec![
            Call(pop),
            TeeLocal(1), // store dividend for later (TODO: check for divide by 0)
            Call(pop),
            TeeLocal(0), // store divisor as well

            // To find the "real" mod, check if the signs mismatch
            I32Xor,
            I32Const(i32::MIN), // 0x80000000 (high bit set)
            I32And,

            If(BlockType::Value(ValueType::I32)),
            // WASM has round-towards-0 semantics, forth has round-negative.
            // Add the dividend to the remainder to get the mod.
            GetLocal(0),
            GetLocal(1),
            I32RemS,
            GetLocal(1),
            I32Add,

            Else, // If the signs match, the remainder IS the mod.
            GetLocal(0),
            GetLocal(1),
            I32RemS,
            End,

            Call(push),
        ]);

        self.define_native_word("AND", 0, binary_i32(I32And));
        self.define_native_word("OR", 0, binary_i32(I32Or));
        self.define_native_word(
            "INVERT",
            0,
            vec![
                I32Const(-1),
                I32Const(0),
                Call(pop),
                I32Eqz,
                Select,
                Call(push),
            ],
        );

        self.define_native_word("=", 0, binary_bool(I32Eq));
        self.define_native_word("<>", 0, binary_bool(I32Ne));
        self.define_native_word("<", 0, binary_bool(I32LtS));
        self.define_native_word(">", 0, binary_bool(I32GtS));
        self.define_native_word("<=", 0, binary_bool(I32LeS));
        self.define_native_word(">=", 0, binary_bool(I32GeS));
        self.define_native_word(
            "=0",
            0,
            vec![I32Const(0), Call(pop), I32Eqz, I32Sub, Call(push)],
        );
        self.define_native_word(
            "<0",
            0,
            vec![
                I32Const(0),
                Call(pop),
                I32Const(0),
                I32LtS,
                I32Sub,
                Call(push),
            ],
        );
        self.define_native_word(
            ">0",
            0,
            vec![
                I32Const(0),
                Call(pop),
                I32Const(0),
                I32GtS,
                I32Sub,
                Call(push),
            ],
        );
    }

    fn finalize(mut self) -> Self {
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
            let func = self.compiler.add_func(
                vec![],
                vec![],
                vec![],
                vec![I32Const(xt), Call(execute), End],
            );
            self.compiler.add_export(&word, |e| e.func(func));
        }
        self
    }

    fn define_native_word(&mut self, name: &str, locals: usize, instructions: Vec<Instruction>) {
        let code = self.create_native_callable(locals, instructions);
        self.define_word(name, code, &[]);
    }

    fn create_native_callable(&mut self, locals: usize, mut instructions: Vec<Instruction>) -> u32 {
        instructions.push(End);
        let locals = vec![ValueType::I32; locals];
        let func = self
            .compiler
            .add_func(vec![ValueType::I32], vec![], locals, instructions);
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
        .initialize()
    }
}
