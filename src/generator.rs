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
    start: u32,
    ip: u32,
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
        // track the end of the dictionary as we go, to turn relative jumps absolute
        let mut cp = self.cp + 1 + name.len() as i32 + 4 + 4;
        for value in values {
            match value {
                ColonValue::XT(name) => {
                    cp += 4;
                    let xt = self.get_execution_token(name);
                    bytes.extend_from_slice(&xt.to_le_bytes())
                }
                ColonValue::Lit(value) => {
                    cp += 8;
                    bytes.extend_from_slice(&lit_xt.to_le_bytes());
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                ColonValue::Branch(offset) => {
                    cp += 8;
                    let target = cp + offset;
                    bytes.extend_from_slice(&branch_xt.to_le_bytes());
                    bytes.extend_from_slice(&target.to_le_bytes());
                }
                ColonValue::QBranch(offset) => {
                    cp += 8;
                    let target = cp + offset;
                    bytes.extend_from_slice(&q_branch_xt.to_le_bytes());
                    bytes.extend_from_slice(&target.to_le_bytes());
                }
            }
        }
        let exit_xt = self.get_execution_token("EXIT");
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

        // Define CP and LAST-WORD variables now
        // We don't have their real values yet, but other code needs to reference them
        self.define_variable_word("CP", 0);
        self.define_variable_word("LAST-WORD", 0);
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
            let push =
                compiler.add_native_func(vec![ValueType::I32], vec![], vec![], push_instructions);

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
            let pop =
                compiler.add_native_func(vec![], vec![ValueType::I32], vec![], pop_instructions);

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
        self.define_native_word(
            "NIP",
            0,
            vec![
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
                TeeLocal(0), // point to head - 4
                GetLocal(0),
                I32Load(2, 4),  // retrieve value of head
                I32Store(2, 0), // store in head - 4
                GetLocal(0),
                SetGlobal(stack), // head -= 4
            ],
        );
        self.define_native_word(
            "TUCK",
            1,
            vec![
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
                TeeLocal(0), // save head - 4
                I32Load(2, 4),
                SetLocal(1), // save [head]
                // start moving
                GetLocal(0),
                GetLocal(0),
                I32Load(2, 0),
                I32Store(2, 4), // store [head - 4] in head
                GetLocal(0),
                GetLocal(1),
                I32Store(2, 0), // store old [head] in head - 4
                GetLocal(0),
                GetLocal(1),
                I32Store(2, 8), // store old [head] in head + 4
                // and just increment the stack ptr and we're done
                GetLocal(0),
                I32Const(8),
                I32Add,
                SetGlobal(stack),
            ],
        );
        self.define_native_word(
            "ROT",
            0,
            vec![
                // spin your elements round and round
                GetGlobal(stack),
                I32Const(8),
                I32Sub,
                TeeLocal(0),
                GetLocal(0),
                I32Load(2, 0),
                GetLocal(0),
                GetLocal(0),
                I32Load(2, 4),
                GetLocal(0),
                GetLocal(0),
                I32Load(2, 8),
                I32Store(2, 4),
                I32Store(2, 0),
                I32Store(2, 8),
            ],
        );
        self.define_native_word(
            "-ROT",
            0,
            vec![
                // like two rots, or rot backwards
                GetGlobal(stack),
                I32Const(8),
                I32Sub,
                TeeLocal(0),
                GetLocal(0),
                I32Load(2, 0),
                GetLocal(0),
                GetLocal(0),
                I32Load(2, 4),
                GetLocal(0),
                GetLocal(0),
                I32Load(2, 8),
                I32Store(2, 0),
                I32Store(2, 8),
                I32Store(2, 4),
            ],
        );
        self.define_native_word(">R", 0, vec![Call(pop), Call(push_r)]);
        self.define_native_word("R>", 0, vec![Call(pop_r), Call(push)]);
        self.define_native_word("R@", 0, vec![GetGlobal(r_stack), I32Load(2, 0), Call(push)]);
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
        self.define_constant_word("(DOCON)", docon as i32);
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
        self.define_constant_word("(DOVAR)", dovar as i32);
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

        let ip = self.add_global(0);
        self.ip = ip;
        let stopped = self.add_global(0);

        // "execute" takes an XT as a parameter and runs it
        let execute = self.compiler.add_native_func(
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
        self.define_native_word("EXECUTE", 0, vec![Call(pop), Call(execute)]);

        // Start is the interpreter's main loop, it calls EXECUTE until the program says to stop.
        // Assuming that the caller has set IP to something reasonable first.
        let start = self.compiler.add_native_func(
            vec![],
            vec![],
            vec![],
            vec![
                // mark that we should NOT stop yet
                I32Const(0),
                SetGlobal(stopped),
                // loop until execution is not "in progress"
                Loop(BlockType::NoResult),
                GetGlobal(ip), // IP is a pointer to an XT
                I32Load(2, 0), // Deref it to get our next XT
                Call(execute), // Run it
                GetGlobal(ip),
                I32Const(4),
                I32Add,
                SetGlobal(ip), // increment the IP
                // loop if we still have not been stopped
                GetGlobal(stopped),
                I32Eqz,
                BrIf(0),
                End,
                End,
            ],
        );
        self.start = start;
        self.define_native_word("STOP", 0, vec![I32Const(-1), SetGlobal(stopped)]);

        // DOCOL is how a colon word is executed. It just messes with the IP.
        let docol = self.create_native_callable(
            0,
            vec![
                // push IP onto the return stack
                GetGlobal(ip),
                Call(push_r),
                // Set IP to the head of our parameter
                GetLocal(0),
                I32Const(4),
                I32Sub,
                SetGlobal(ip),
            ],
        );
        self.docol = docol;
        self.define_constant_word("(DOCOL)", docol as i32);
        // EXIT is how a colon word returns. It just restores the old IP.
        self.define_native_word(
            "EXIT",
            0,
            vec![
                // Set IP to whatever's the head of the return stack
                Call(pop_r),
                SetGlobal(ip),
            ],
        );
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
                // The value after that is a literal jump address, jump there
                GetGlobal(ip),
                I32Load(2, 4),
                I32Const(4), // Subtract 4 to account for the main loop incrementing the IP itself
                I32Sub,
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
                // Jump to literal-after-the-IP - 4
                GetGlobal(ip),
                I32Load(2, 4),
                I32Const(4),
                I32Sub,
                Else, // Just jump to 4-after-the-IP
                GetGlobal(ip),
                I32Const(4),
                I32Add,
                End,
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

        self.define_native_word(
            "MIN",
            1,
            vec![
                Call(pop),
                TeeLocal(0),
                Call(pop),
                TeeLocal(1),
                GetLocal(0),
                GetLocal(1),
                I32LtS,
                Select,
                Call(push),
            ],
        );
        self.define_native_word(
            "MAX",
            1,
            vec![
                Call(pop),
                TeeLocal(0),
                Call(pop),
                TeeLocal(1),
                GetLocal(0),
                GetLocal(1),
                I32GtS,
                Select,
                Call(push),
            ],
        );

        self.define_native_word("1+", 0, vec![Call(pop), I32Const(1), I32Add, Call(push)]);
        self.define_native_word("1-", 0, vec![Call(pop), I32Const(1), I32Sub, Call(push)]);

        self.define_native_word("AND", 0, binary_i32(I32And));
        self.define_native_word("OR", 0, binary_i32(I32Or));

        self.define_constant_word("FALSE", 0);
        self.define_constant_word("TRUE", -1);
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
            "<>0",
            0,
            vec![
                I32Const(0),
                Call(pop),
                I32Const(0),
                I32Ne,
                I32Sub,
                Call(push),
            ],
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
        // For testing purposes, define a word that just calls another word and stops.
        self.define_colon_word(
            "RUN-WORD",
            vec![ColonValue::XT("EXECUTE"), ColonValue::XT("STOP")],
        );

        // Now that we're done adding things to the dictionary,
        // set values for CP (a var containing the next address in the dictionary)
        // and LAST-WORD (a var containing the address of the final word).

        let cp_storage_address = self.get_execution_token("CP") + 4;
        let cp_bytes: Vec<u8> = self.cp.to_le_bytes().iter().copied().collect();
        self.compiler.add_data(cp_storage_address, cp_bytes);

        let last_word_storage_address = self.get_execution_token("LAST-WORD") + 4;
        let last_word_bytes = self
            .last_word_address
            .to_le_bytes()
            .iter()
            .copied()
            .collect();
        self.compiler
            .add_data(last_word_storage_address, last_word_bytes);

        // For testing, export every word as a function-which-EXECUTEs-that-word
        let run_xt = self.get_execution_token("RUN-WORD");
        for (word, xt) in self.execution_tokens.clone() {
            let func = self.compiler.add_native_func(
                vec![],
                vec![],
                vec![],
                vec![
                    I32Const(xt),
                    Call(self.push), // Add the function to call onto the stack
                    I32Const(run_xt + 4),
                    SetGlobal(self.ip), // Set the IP to within the "run this" func
                    Call(self.start),   // start the main loop
                    End,
                ],
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
        let func =
            self.compiler
                .add_native_func(vec![ValueType::I32], vec![], locals, instructions);
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
            start: 0,
            ip: 0,
            cp: 0x1000,
            last_word_address: 0,
            execution_tokens: HashMap::new(),
        }
        .initialize()
    }
}
