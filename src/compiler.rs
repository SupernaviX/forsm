use super::assembler::Assembler;
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

pub struct Compiler {
    assembler: Assembler,
    stack: u32,
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

impl Compiler {
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

    pub fn define_imported_word(
        &mut self,
        module: &str,
        name: &str,
        params: usize,
        results: usize,
    ) {
        // Define an imported with the given signature, but a lowercase name
        let field = name.to_ascii_lowercase();
        let func = self.assembler.add_imported_func(
            module.to_owned(),
            field,
            vec![ValueType::I32; params],
            vec![ValueType::I32; results],
        );

        // Define a native word to call the import using the stack
        let locals = if results == 0 { 0 } else { 1 };
        let mut instructions = vec![];
        if params > 0 {
            // get the lowest parameter address into local 0 and onto the stack
            instructions.push(GetGlobal(self.stack));
            if params > 1 {
                instructions.push(I32Const((params as i32 - 1) * 4));
                instructions.push(I32Sub);
            }
            instructions.push(TeeLocal(0));
            // pass parameters in LIFO order, so stack effects match function signatures
            for param in 0..params {
                if param > 0 {
                    instructions.push(GetLocal(0));
                }
                instructions.push(I32Load(2, param as u32 * 4));
            }
        }
        instructions.push(Call(func));
        if results > 0 {
            // get the lowest result address in local 0 and on the stack
            if params == 0 {
                instructions.push(GetGlobal(self.stack));
                instructions.push(I32Const(4));
                instructions.push(I32Add);
                instructions.push(SetLocal(0));
            }
            // store results in FIFO order, also so stack effects match signatures
            for result in 0..results {
                instructions.push(SetLocal(1));
                instructions.push(GetLocal(0));
                instructions.push(GetLocal(1));
                instructions.push(I32Store(2, (results - 1 - result) as u32 * 4))
            }
        }
        if params != results {
            // move the stack pointer appropriately.
            // We already have the "lowest" address stored in local 0,
            // just offset that by the size of results and store it
            instructions.push(GetLocal(0));
            if results != 1 {
                instructions.push(I32Const((results as i32 - 1) * 4));
                instructions.push(I32Add);
            }
            instructions.push(SetGlobal(self.stack));
        }
        self.define_native_word(name, locals, instructions);
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        self.finalize().assembler.compile()
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
        let define_stack = |assembler: &mut Assembler, stack| {
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
                assembler.add_native_func(vec![ValueType::I32], vec![], vec![], push_instructions);

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
                assembler.add_native_func(vec![], vec![ValueType::I32], vec![], pop_instructions);

            (push, pop)
        };
        // define the normal stack
        let stack = self.add_global(0x00);
        let (push, pop) = define_stack(&mut self.assembler, stack);
        self.stack = stack;
        self.push = push;
        self.pop = pop;
        // define the return stack
        let r_stack = self.add_global(0x80);
        let (push_r, pop_r) = define_stack(&mut self.assembler, r_stack);
        self.push_r = push_r;
        self.pop_r = pop_r;

        self.assembler.add_exported_func("push", push);
        self.assembler.add_exported_func("pop", pop);
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
        let execute = self.assembler.add_native_func(
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
        let start = self.assembler.add_native_func(
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
        self.assembler.add_data(cp_storage_address, cp_bytes);

        let last_word_storage_address = self.get_execution_token("LAST-WORD") + 4;
        let last_word_bytes = self
            .last_word_address
            .to_le_bytes()
            .iter()
            .copied()
            .collect();
        self.assembler
            .add_data(last_word_storage_address, last_word_bytes);

        // For testing, export every word as a function-which-EXECUTEs-that-word
        let run_xt = self.get_execution_token("RUN-WORD");
        for (word, xt) in self.execution_tokens.clone() {
            let func = self.assembler.add_native_func(
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
            self.assembler.add_exported_func(&word, func);
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
            self.assembler
                .add_native_func(vec![ValueType::I32], vec![], locals, instructions);
        self.assembler.add_table_entry(func)
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
        self.assembler.add_data(self.cp, data);
        self.cp = cp;
        self.last_word_address = last_word_address;
    }

    fn add_global(&mut self, initial_value: i32) -> u32 {
        self.assembler.add_global(|g| {
            g.mutable()
                .value_type()
                .i32()
                .init_expr(I32Const(initial_value))
        })
    }
}
impl Default for Compiler {
    fn default() -> Self {
        let mut assembler: Assembler = Default::default();
        assembler.add_memory();
        Self {
            assembler,
            stack: 0,
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use wasmer::{imports, Function, ImportObject, Store};

    use super::{ColonValue::*, Compiler};
    use crate::runtime::Runtime;

    fn build<T>(func: T) -> Result<Runtime>
    where
        T: FnOnce(&mut Compiler),
    {
        build_with_imports(func, |_| imports! {})
    }

    fn build_with_imports<T, F>(func: T, imports: F) -> Result<Runtime>
    where
        T: FnOnce(&mut Compiler),
        F: FnOnce(&Store) -> ImportObject,
    {
        let mut compiler = Compiler::default();
        func(&mut compiler);
        let binary = compiler.compile()?;
        Runtime::new(&binary, imports)
    }

    #[test]
    fn should_manipulate_stack() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();

        assert_eq!(runtime.pop().unwrap(), 3);
        assert_eq!(runtime.pop().unwrap(), 2);
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_do_math() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(3).unwrap();
        runtime.push(4).unwrap();
        runtime.execute("+").unwrap();

        assert_eq!(runtime.pop().unwrap(), 7);
    }

    #[test]
    fn should_do_division() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(6).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("/").unwrap();

        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_do_comparisons() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(2).unwrap();
        runtime.push(1).unwrap();
        runtime.execute(">").unwrap();
        assert_eq!(runtime.pop().unwrap(), -1);

        runtime.push(1).unwrap();
        runtime.execute("<0").unwrap();
        assert_eq!(runtime.pop().unwrap(), 0);
    }

    #[test]
    fn should_handle_signed_div_and_mod() {
        let runtime = build(|_| {}).unwrap();
        type TestCase = ((i32, i32), (i32, i32));

        let test_cases: Vec<TestCase> = vec![
            ((7, 4), (1, 3)),
            ((-7, 4), (-2, 1)),
            ((7, -4), (-2, -1)),
            ((-7, -4), (1, -3)),
        ];

        let results: Vec<TestCase> = test_cases
            .iter()
            .map(|case| {
                let ((divisor, dividend), _) = *case;

                runtime.push(divisor).unwrap();
                runtime.push(dividend).unwrap();
                runtime.execute("/").unwrap();
                let quotient = runtime.pop().unwrap();

                runtime.push(divisor).unwrap();
                runtime.push(dividend).unwrap();
                runtime.execute("MOD").unwrap();
                let modulo = runtime.pop().unwrap();

                ((divisor, dividend), (quotient, modulo))
            })
            .collect();
        assert_eq!(results, test_cases);
    }

    #[test]
    fn should_support_colon_words() {
        let runtime = build(|compiler| {
            compiler.define_colon_word("TEST", vec![Lit(2), Lit(3), XT("+")]);
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 5);
    }

    #[test]
    fn should_support_variables() {
        let runtime = build(|compiler| {
            compiler.define_variable_word("TESTVAR", 0);
            compiler.define_colon_word(
                "TEST",
                vec![Lit(1), XT("TESTVAR"), XT("!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_increment_variables() {
        let runtime = build(|compiler| {
            compiler.define_variable_word("TESTVAR", 6);
            compiler.define_colon_word(
                "TEST",
                vec![Lit(7), XT("TESTVAR"), XT("+!"), XT("TESTVAR"), XT("@")],
            );
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 13);
    }

    #[test]
    fn should_dup() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.execute("DUP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_swap() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.execute("SWAP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_rot() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("ROT").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 3);
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_backwards_rot() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("-ROT").unwrap();
        assert_eq!(runtime.pop().unwrap(), 2);
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 3);
    }

    #[test]
    fn should_nip() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("NIP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 3);
        assert_eq!(runtime.pop().unwrap(), 1);
    }

    #[test]
    fn should_tuck() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.execute("TUCK").unwrap();
        assert_eq!(runtime.pop().unwrap(), 2);
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_support_literals() {
        let runtime = build(|compiler| {
            compiler.define_colon_word("THREE", vec![Lit(3)]);
        })
        .unwrap();

        runtime.execute("THREE").unwrap();
        assert_eq!(runtime.pop().unwrap(), 3);
    }

    #[test]
    fn should_support_stack_manip() {
        let runtime = build(|compiler| {
            compiler.define_colon_word(
                "TEST",
                vec![Lit(3), XT("DUP"), XT("DUP"), XT("+"), XT("SWAP"), XT("/")],
            );
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_support_nested_colon_calls() {
        let runtime = build(|compiler| {
            compiler.define_colon_word("SQUARE", vec![XT("DUP"), XT("*")]);
            compiler.define_colon_word("TEST", vec![Lit(3), XT("SQUARE")]);
        })
        .unwrap();
        runtime.execute("TEST").unwrap();
        assert_eq!(runtime.pop().unwrap(), 9);
    }

    #[test]
    fn should_support_branching() {
        let runtime = build(|compiler| {
            #[rustfmt::skip]
            compiler.define_colon_word("UPCHAR", vec![
                XT("DUP"), XT("DUP"),
                Lit(97), XT(">="), XT("SWAP"), Lit(122), XT("<="), XT("AND"),
                QBranch(12), // Lit(32) is 8 bytes, XT("-") is 4
                Lit(32), XT("-"),
            ]);
        })
        .unwrap();

        runtime.push('a' as i32).unwrap();
        runtime.execute("UPCHAR").unwrap();
        assert_eq!(runtime.pop().unwrap(), 'A' as i32);

        runtime.push('B' as i32).unwrap();
        runtime.execute("UPCHAR").unwrap();
        assert_eq!(runtime.pop().unwrap(), 'B' as i32);
    }

    #[test]
    fn should_support_imports() {
        let runtime = build_with_imports(
            |compiler| {
                compiler.define_imported_word("test", "SEVENTEEN", 0, 2);
                compiler.define_imported_word("test", "SWALLOW", 2, 0);
                compiler.define_imported_word("test", "TRIM", 2, 2);
            },
            |store| {
                imports! {
                    "test" => {
                        "seventeen" => Function::new_native(store, || (10, 7)),
                        "swallow" => Function::new_native(store, |_: i32, _: i32| {}),
                        "trim" => Function::new_native(store, |a: i32, b: i32| {
                            (a + 4, b - 8)
                        }),
                    }
                }
            },
        )
        .unwrap();

        runtime.execute("SEVENTEEN").unwrap();
        runtime.execute("+").unwrap();
        assert_eq!(runtime.pop().unwrap(), 17);

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.push(3).unwrap();
        runtime.execute("SWALLOW").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);

        runtime.push(0).unwrap();
        runtime.push(16).unwrap();
        runtime.execute("TRIM").unwrap();
        assert_eq!(runtime.pop().unwrap(), 8);
        assert_eq!(runtime.pop().unwrap(), 4);
    }
}
