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
    StringLit(String),
    Branch(i32),
    QBranch(i32),
}

#[derive(Clone, Copy)]
pub enum ParamType {
    I32,
    I64,
}
impl ParamType {
    pub(crate) fn bytes(&self) -> u32 {
        match self {
            ParamType::I32 => 4,
            ParamType::I64 => 8,
        }
    }
    pub(crate) fn load(&self, offset: u32, instructions: &mut Vec<Instruction>) {
        match self {
            ParamType::I32 => instructions.push(I32Load(2, offset)),
            ParamType::I64 => {
                instructions.push(I64Load(2, offset));
                instructions.push(I64Const(32));
                instructions.push(I64Rotl);
            }
        }
    }
    pub(crate) fn store(&self, offset: u32, instructions: &mut Vec<Instruction>) {
        match self {
            ParamType::I32 => instructions.push(I32Store(2, offset)),
            ParamType::I64 => {
                instructions.push(I64Const(32));
                instructions.push(I64Rotl);
                instructions.push(I64Store(2, offset));
            }
        }
    }
}

pub struct Compiler {
    assembler: Assembler,
    stack: u32,
    push: u32,
    pop: u32,
    push_d: u32,
    pop_d: u32,
    push_r: u32,
    pop_r: u32,
    docon: u32,
    dovar: u32,
    docol: u32,
    start: u32,
    ip: u32,
    cp: i32,
    latest_address: i32,
    execution_tokens: HashMap<String, i32>,
}

const DICTIONARY_BASE: i32 = 0x1000;
const PARAM_STACK_BASE: i32 = 0xef00;
const RETURN_STACK_BASE: i32 = 0xf000;
const HEAP_BASE: i32 = 0xf100;

const DICTIONARY_CAPACITY: i32 = PARAM_STACK_BASE - DICTIONARY_BASE;

const ALIGNMENT: i32 = 4;

fn required_padding(offset: i32) -> i32 {
    -offset & (ALIGNMENT - 1)
}
fn aligned(offset: i32) -> i32 {
    offset + required_padding(offset)
}
fn header_size(name: &str) -> i32 {
    aligned(1 + name.len() as i32) + 4 + 4
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
        let mut cp = self.cp + header_size(name);
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
                ColonValue::StringLit(value) => {
                    let data_start = cp + 8;
                    let data_len = value.len() as i32;
                    let padding = required_padding(data_len);
                    let target = data_start + data_len + padding;
                    cp = target + 16;
                    bytes.extend_from_slice(&branch_xt.to_le_bytes());
                    bytes.extend_from_slice(&target.to_le_bytes());
                    bytes.extend_from_slice(value.as_bytes());
                    bytes.extend_from_slice(&vec![0; padding as usize]);
                    bytes.extend_from_slice(&lit_xt.to_le_bytes());
                    bytes.extend_from_slice(&data_start.to_le_bytes());
                    bytes.extend_from_slice(&lit_xt.to_le_bytes());
                    bytes.extend_from_slice(&data_len.to_le_bytes());
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
        name: &str,
        module: &str,
        field: &str,
        params: Vec<ParamType>,
        results: Vec<ParamType>,
    ) {
        let to_value_types = |types: &[ParamType]| -> Vec<ValueType> {
            types
                .iter()
                .map(|t| match t {
                    ParamType::I32 => ValueType::I32,
                    ParamType::I64 => ValueType::I64,
                })
                .collect()
        };
        // Define an imported with the given signature
        let func = self.assembler.add_imported_func(
            module.to_owned(),
            field.to_owned(),
            to_value_types(&params),
            to_value_types(&results),
        );
        let params_bytes = params.iter().map(|p| p.bytes()).sum();
        let results_bytes = results.iter().map(|p| p.bytes()).sum();

        // Define a native word to call the import using the stack
        let locals = if results.is_empty() {
            vec![]
        } else {
            vec![ValueType::I64]
        };
        let mut instructions = vec![];
        if !params.is_empty() {
            instructions.push(GetGlobal(self.stack));
            instructions.push(TeeLocal(0));
            let mut param_offset = params_bytes;
            // pass parameters in LIFO order, so stack effects match function signatures
            for (param, _type) in params.iter().enumerate() {
                if param > 0 {
                    instructions.push(GetLocal(0));
                }
                param_offset -= _type.bytes();
                _type.load(param_offset, &mut instructions);
            }
        }
        instructions.push(Call(func));
        // If the stack size has changed, move the stack pointer appropriately
        if params_bytes != results_bytes {
            if !params.is_empty() {
                instructions.push(GetLocal(0));
            } else {
                instructions.push(GetGlobal(self.stack));
            }
            let delta = params_bytes as i32 - results_bytes as i32;
            instructions.push(I32Const(delta));
            instructions.push(I32Add);
            if !results.is_empty() {
                // hold onto the new stack head so we can write results
                instructions.push(TeeLocal(0));
            }
            instructions.push(SetGlobal(self.stack));
        }
        // store results in FIFO order, also so stack effects can match signatures
        // at this point, local 0 holds the head (lowest address) of the stack
        let mut result_offset = 0;
        for _type in results.iter().rev() {
            let local = match _type {
                ParamType::I32 => 1,
                ParamType::I64 => 2,
            };
            instructions.push(SetLocal(local));
            instructions.push(GetLocal(0));
            instructions.push(GetLocal(local));
            _type.store(result_offset, &mut instructions);
            result_offset += _type.bytes();
        }
        self.define_native_word(name, locals, instructions);
    }

    pub fn compile(self) -> Result<Vec<u8>> {
        self.finalize().assembler.compile()
    }

    fn initialize(mut self) -> Self {
        self.define_stacks();
        self.define_memory();
        self.define_execution();
        self.define_math();

        // Define dictionary-related words here as well
        // We don't have some real values yet, but other code needs to reference them
        self.define_constant_word("DICT-BASE", DICTIONARY_BASE);
        self.define_constant_word("DICT-CAPACITY", DICTIONARY_CAPACITY);
        self.define_variable_word("CP", DICTIONARY_BASE);
        self.define_variable_word("LATEST", DICTIONARY_BASE);
        self
    }

    fn define_stacks(&mut self) {
        let define_stack = |assembler: &mut Assembler, stack| {
            let push_instructions = vec![
                // decrement stack pointer
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
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
                // increment stack pointer
                GetGlobal(stack),
                I32Const(4),
                I32Add,
                SetGlobal(stack),
                End,
            ];
            let pop =
                assembler.add_native_func(vec![], vec![ValueType::I32], vec![], pop_instructions);

            (push, pop)
        };
        // define the normal stack
        let stack = self.add_global(PARAM_STACK_BASE);
        let (push, pop) = define_stack(&mut self.assembler, stack);
        self.stack = stack;
        self.push = push;
        self.pop = pop;

        let push_d = self.assembler.add_native_func(
            vec![ValueType::I64],
            vec![],
            vec![],
            vec![
                // decrement stack pointer
                GetGlobal(stack),
                I32Const(8),
                I32Sub,
                SetGlobal(stack),
                // write data
                GetGlobal(stack),
                GetLocal(0),
                I64Const(32),
                I64Rotl,
                I64Store(3, 0),
                End,
            ],
        );
        let pop_d = self.assembler.add_native_func(
            vec![],
            vec![ValueType::I64],
            vec![],
            vec![
                // read data
                GetGlobal(stack),
                I64Load(3, 0),
                I64Const(32),
                I64Rotl,
                // increment stack pointer
                GetGlobal(stack),
                I32Const(8),
                I32Add,
                SetGlobal(stack),
                End,
            ],
        );
        self.push_d = push_d;
        self.pop_d = pop_d;

        // define the return stack
        let r_stack = self.add_global(RETURN_STACK_BASE);
        let (push_r, pop_r) = define_stack(&mut self.assembler, r_stack);
        self.push_r = push_r;
        self.pop_r = pop_r;

        #[cfg(test)]
        {
            self.assembler.add_exported_func("push", push);
            self.assembler.add_exported_func("pop", pop);
            self.assembler.add_exported_func("push_d", push_d);
            self.assembler.add_exported_func("pop_d", pop_d);
        }
        self.define_native_word(
            "DUP",
            vec![],
            vec![
                // just push the top of the stack onto itself
                GetGlobal(stack),
                I32Load(2, 0),
                Call(push),
            ],
        );
        self.define_native_word(
            "?DUP",
            vec![],
            vec![
                GetGlobal(stack),
                I32Load(2, 0),
                TeeLocal(0),
                If(BlockType::NoResult),
                GetLocal(0),
                Call(push),
                End,
            ],
        );
        self.define_native_word(
            "2DUP",
            vec![],
            vec![
                GetGlobal(stack),
                I32Const(8),
                I32Sub,
                TeeLocal(0),
                SetGlobal(stack), // reserve room for two new words
                GetLocal(0),
                GetLocal(0),
                I64Load(3, 8),
                I64Store(3, 0),
            ],
        );
        self.define_native_word(
            "DROP",
            vec![],
            vec![
                // just increment the stack pointer
                GetGlobal(stack),
                I32Const(4),
                I32Add,
                SetGlobal(stack),
            ],
        );
        self.define_native_word(
            "2DROP",
            vec![],
            vec![
                // just increment the stack pointer
                GetGlobal(stack),
                I32Const(8),
                I32Add,
                SetGlobal(stack),
            ],
        );
        self.define_native_word(
            "SWAP",
            vec![],
            vec![
                // don't bother touching the stack pointer
                GetGlobal(stack),
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
            "2SWAP",
            vec![],
            vec![
                // don't bother touching the stack pointer
                GetGlobal(stack),
                TeeLocal(0),
                GetLocal(0),
                I64Load(3, 0),
                GetLocal(0),
                GetLocal(0),
                I64Load(3, 8),
                I64Store(3, 0),
                I64Store(3, 8),
            ],
        );
        self.define_native_word(
            "OVER",
            vec![],
            vec![
                GetGlobal(stack),
                I32Const(4),
                I32Sub,
                TeeLocal(0),
                SetGlobal(stack),
                GetLocal(0),
                GetLocal(0),
                I32Load(2, 8),
                I32Store(2, 0),
            ],
        );
        self.define_native_word(
            "2OVER",
            vec![],
            vec![
                GetGlobal(stack),
                I32Const(8),
                I32Sub,
                TeeLocal(0),
                SetGlobal(stack),
                GetLocal(0),
                GetLocal(0),
                I64Load(2, 16),
                I64Store(2, 0),
            ],
        );
        self.define_native_word(
            "NIP",
            vec![],
            vec![
                GetGlobal(stack),
                TeeLocal(0),
                GetLocal(0),
                I32Load(2, 0),  // retrieve value of head
                I32Store(2, 4), // store in head + 4
                GetLocal(0),
                I32Const(4),
                I32Add,
                SetGlobal(stack), // head += 4
            ],
        );
        self.define_native_word(
            "TUCK",
            vec![],
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
                I32Load(2, 8),
                I32Store(2, 4), // store [head + 4] in head
                GetLocal(0),
                GetLocal(1),
                I32Store(2, 8), // store old [head] in head + 4
                GetLocal(0),
                GetLocal(1),
                I32Store(2, 0), // store old [head] in head - 4
                // and just save the new stack ptr and we're done
                GetLocal(0),
                SetGlobal(stack),
            ],
        );
        self.define_native_word(
            "ROT",
            vec![],
            vec![
                // spin your elements round and round
                GetGlobal(stack),
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
        self.define_native_word(
            "-ROT",
            vec![],
            vec![
                // like two rots, or rot backwards
                GetGlobal(stack),
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
            "PICK",
            vec![],
            vec![
                GetGlobal(stack),
                TeeLocal(0),
                GetLocal(0),
                I32Load(2, 0), // read the head of the stack
                I32Const(1),
                I32Add, // + to account for the address itself at the top of the stack
                I32Const(2),
                I32Shl, // * 4 to make it an offset
                GetLocal(0),
                I32Add,
                I32Load(2, 0),  // read that offset from the head
                I32Store(2, 0), // and store it back in the head
            ],
        );
        self.define_native_word(
            "DEPTH",
            vec![],
            vec![
                I32Const(PARAM_STACK_BASE),
                GetGlobal(stack),
                I32Sub,
                I32Const(2),
                I32ShrU,
                Call(push),
            ],
        );
        self.define_native_word(">R", vec![], vec![Call(pop), Call(push_r)]);
        self.define_native_word("R>", vec![], vec![Call(pop_r), Call(push)]);
        self.define_native_word(
            "R@",
            vec![],
            vec![GetGlobal(r_stack), I32Load(2, 0), Call(push)],
        );
        self.define_native_word(
            "R-DEPTH",
            vec![],
            vec![
                I32Const(RETURN_STACK_BASE),
                GetGlobal(r_stack),
                I32Sub,
                I32Const(2),
                I32ShrU,
                Call(push),
            ],
        );
    }

    fn define_memory(&mut self) {
        let push = self.push;
        let pop = self.pop;

        // constants
        let docon = self.create_native_callable(
            vec![],
            vec![
                // The value of our parameter is the value of the constant, just fetch and push it
                GetLocal(0),
                I32Load(2, 0),
                Call(push),
            ],
        );
        self.docon = docon;
        self.define_constant_word("(DOCON)", docon as i32);

        // variables
        let dovar = self.create_native_callable(
            vec![],
            vec![
                // the address of our parameter IS the address of the variable, just push it
                GetLocal(0),
                Call(push),
            ],
        );
        self.dovar = dovar;
        self.define_constant_word("(DOVAR)", dovar as i32);
        self.define_native_word("!", vec![], vec![Call(pop), Call(pop), I32Store(2, 0)]);
        self.define_native_word("@", vec![], vec![Call(pop), I32Load(2, 0), Call(push)]);
        self.define_native_word(
            "+!",
            vec![],
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
        self.define_native_word("C!", vec![], vec![Call(pop), Call(pop), I32Store8(0, 0)]);
        self.define_native_word("C@", vec![], vec![Call(pop), I32Load8U(0, 0), Call(push)]);

        // heap words
        self.define_constant_word("HEAP-BASE", HEAP_BASE);
        self.define_native_word("MEMORY.SIZE", vec![], vec![CurrentMemory(0), Call(push)]);
        self.define_native_word(
            "MEMORY.GROW",
            vec![],
            vec![Call(pop), GrowMemory(0), Call(push)],
        );
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
        let callable_sig = self
            .assembler
            .add_type(vec![ValueType::I32, ValueType::I32], vec![]);
        let execute = self.assembler.add_native_func(
            vec![ValueType::I32],
            vec![],
            vec![ValueType::I32],
            vec![
                // The argument is an execution token (XT).
                // In this system, an execution token is a 32-bit address.
                // The low 8 bits of the value are a table index,
                // and the rest are a 24-bit "immediate" value.
                // Any parameter data is stored immediately after it.
                // Call the func with:
                // arg0: the address of the parameter data.
                // arg1: the immediate
                GetLocal(0),
                I32Const(4),
                I32Add,
                GetLocal(0),
                I32Load(2, 0),
                TeeLocal(1),
                I32Const(8),
                I32ShrU, // top 24 bits are arg1
                GetLocal(1),
                I32Const(255),
                I32And, // bottom 8 bits are the func index
                CallIndirect(callable_sig, 0),
                End,
            ],
        );
        self.define_native_word("EXECUTE", vec![], vec![Call(pop), Call(execute)]);

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
        self.define_native_word("BYE", vec![], vec![I32Const(-1), SetGlobal(stopped)]);

        // DOCOL is how a colon word is executed. It just messes with the IP.
        let docol = self.create_native_callable(
            vec![],
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
            vec![],
            vec![
                // Set IP to whatever's the head of the return stack
                Call(pop_r),
                SetGlobal(ip),
            ],
        );

        // DODOES is what lets you customize runtime behavior of a word.
        // It does what DOCOL does, except it also pushes a word onto the stack.
        let dodoes = self.create_native_callable(
            vec![],
            vec![
                // push the head of our parameter onto the stack
                GetLocal(0),
                Call(push),
                // push IP onto the return stack
                GetGlobal(ip),
                Call(push_r),
                // Set IP to our immediate
                GetLocal(1),
                I32Const(4),
                I32Sub,
                SetGlobal(ip),
            ],
        );
        self.define_constant_word("(DODOES)", dodoes as i32);

        self.define_native_word(
            "LIT",
            vec![],
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
            vec![],
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
            vec![],
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
        let push_d = self.push_d;
        let pop_d = self.pop_d;
        let get_two_i32_args = || {
            vec![
                //swap the top of the stack before calling the real ops
                Call(pop),
                SetLocal(0),
                Call(pop),
                GetLocal(0),
            ]
        };
        let get_two_i64_args = || {
            vec![
                //swap the top of the stack before calling the real ops
                Call(pop_d),
                SetLocal(2),
                Call(pop_d),
                GetLocal(2),
            ]
        };
        let binary_i32 = |op| {
            let mut res = get_two_i32_args();
            res.push(op);
            res.push(Call(push));
            res
        };
        let binary_i64 = |op| {
            let mut res = get_two_i64_args();
            res.push(op);
            res.push(Call(push_d));
            res
        };
        let binary_i32_bool = |op| {
            let mut res = vec![I32Const(0)];
            res.extend_from_slice(&get_two_i32_args());
            res.push(op);
            res.push(I32Sub);
            res.push(Call(push));
            res
        };
        let binary_i64_bool = |op| {
            let mut res = vec![I32Const(0)];
            res.extend_from_slice(&get_two_i64_args());
            res.push(op);
            res.push(I32Sub);
            res.push(Call(push));
            res
        };
        self.define_native_word("+", vec![], binary_i32(I32Add));
        self.define_native_word("-", vec![], binary_i32(I32Sub));
        self.define_native_word("*", vec![], binary_i32(I32Mul));

        self.define_native_word(
            "NEGATE",
            vec![],
            vec![I32Const(0), Call(pop), I32Sub, Call(push)],
        );
        self.define_native_word(
            "ABS",
            vec![],
            vec![
                Call(pop),
                TeeLocal(0),
                I32Const(31),
                I32ShrS,
                TeeLocal(1),
                GetLocal(0),
                I32Xor,
                GetLocal(1),
                I32Sub,
                Call(push),
            ],
        );

        self.define_native_word("S>D", vec![], vec![Call(pop), I64ExtendSI32, Call(push_d)]);
        self.define_native_word("D>S", vec![], vec![Call(pop_d), I32WrapI64, Call(push)]);
        self.define_native_word(
            "M+",
            vec![],
            vec![Call(pop), I64ExtendSI32, Call(pop_d), I64Add, Call(push_d)],
        );
        self.define_native_word("D+", vec![ValueType::I64], binary_i64(I64Add));
        self.define_native_word("D-", vec![ValueType::I64], binary_i64(I64Sub));
        self.define_native_word(
            "DABS",
            vec![ValueType::I64, ValueType::I64],
            vec![
                Call(pop_d),
                TeeLocal(2),
                I64Const(63),
                I64ShrS,
                TeeLocal(3),
                GetLocal(2),
                I64Xor,
                GetLocal(3),
                I64Sub,
                Call(push_d),
            ],
        );
        self.define_native_word(
            "DNEGATE",
            vec![],
            vec![I64Const(0), Call(pop_d), I64Sub, Call(push_d)],
        );

        self.define_native_word(
            "M*",
            vec![],
            vec![
                Call(pop),
                SetLocal(0),
                Call(pop),
                I64ExtendSI32,
                GetLocal(0),
                I64ExtendSI32,
                I64Mul,
                Call(push_d),
            ],
        );

        self.define_native_word(
            "UM*",
            vec![],
            vec![
                Call(pop),
                SetLocal(0),
                Call(pop),
                I64ExtendUI32,
                GetLocal(0),
                I64ExtendUI32,
                I64Mul,
                Call(push_d),
            ],
        );

        self.define_native_word(
            "D*",
            vec![],
            vec![
                Call(pop),
                SetLocal(0),
                Call(pop_d),
                GetLocal(0),
                I64ExtendSI32,
                I64Mul,
                Call(push_d),
            ],
        );

        self.define_native_word(
            "UD*",
            vec![],
            vec![
                Call(pop),
                SetLocal(0),
                Call(pop_d),
                GetLocal(0),
                I64ExtendUI32,
                I64Mul,
                Call(push_d),
            ],
        );

        self.define_native_word(
            "UM/MOD",
            vec![ValueType::I64, ValueType::I64],
            vec![
                Call(pop),
                I64ExtendSI32,
                SetLocal(3),
                Call(pop_d),
                TeeLocal(2),
                GetLocal(3),
                I64RemU,
                I32WrapI64,
                Call(push),
                GetLocal(2),
                GetLocal(3),
                I64DivU,
                I32WrapI64,
                Call(push),
            ],
        );

        self.define_native_word(
            "/MOD",
            vec![ValueType::I32, ValueType::I32, ValueType::I32],
            vec![
                Call(pop),
                SetLocal(2),
                Call(pop),
                TeeLocal(1),
                GetLocal(2),
                I32DivS,
                SetLocal(3), // store quotient for now
                GetLocal(1),
                GetLocal(2),
                I32RemS,
                SetLocal(4), // store remainder as well
                // To find the "real" mod, add divisor if numerator+denominator have misnatched signs and remainder <> 0
                GetLocal(4),
                GetLocal(2),
                I32Const(0),
                GetLocal(1),
                GetLocal(2),
                I32Xor,
                I32Const(0),
                I32LtS,
                GetLocal(4),
                I32Const(0),
                I32Ne,
                I32And,
                TeeLocal(0),
                Select,
                I32Add,
                Call(push),
                // To find the "real" quotient, subtract 1 if numerator+denominator have misnatched signs and remainder <> 0
                GetLocal(3),
                I32Const(1),
                I32Const(0),
                GetLocal(0),
                Select,
                I32Sub,
                Call(push),
            ],
        );

        self.define_colon_word("/", vec![ColonValue::XT("/MOD"), ColonValue::XT("NIP")]);
        self.define_colon_word("MOD", vec![ColonValue::XT("/MOD"), ColonValue::XT("DROP")]);

        self.define_native_word(
            "U/MOD",
            vec![],
            vec![
                Call(pop),
                SetLocal(1),
                Call(pop),
                TeeLocal(0),
                GetLocal(1),
                I32RemU,
                Call(push),
                GetLocal(0),
                GetLocal(1),
                I32DivU,
                Call(push),
            ],
        );

        self.define_native_word(
            "SM/REM",
            vec![ValueType::I64, ValueType::I64],
            vec![
                Call(pop),
                I64ExtendSI32,
                SetLocal(3),
                Call(pop_d),
                TeeLocal(2),
                GetLocal(3),
                I64RemS,
                I32WrapI64,
                Call(push),
                GetLocal(2),
                GetLocal(3),
                I64DivS,
                I32WrapI64,
                Call(push),
            ],
        );

        self.define_native_word(
            "FM/MOD",
            vec![
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
            ],
            vec![
                Call(pop),
                I64ExtendSI32,
                SetLocal(3),
                Call(pop_d),
                TeeLocal(2),
                GetLocal(3),
                I64DivS,
                SetLocal(4), // store quotient for now
                GetLocal(2),
                GetLocal(3),
                I64RemS,
                SetLocal(5), // store remainder as well
                // To find the "real" mod, subtract dividend if numerator+denominator have misnatched signs and remainder <> 0
                GetLocal(5),
                GetLocal(2),
                I64Const(0),
                GetLocal(2),
                GetLocal(3),
                I64Xor,
                I64Const(0),
                I64LtS,
                GetLocal(5),
                I64Const(0),
                I64Ne,
                I32And,
                TeeLocal(0),
                Select,
                I64Sub,
                I32WrapI64,
                Call(push),
                // To find the "real" quotient, add 1 if quotient <= 0 and remainder <> 0
                GetLocal(4),
                I64Const(1),
                I64Const(0),
                GetLocal(0),
                Select,
                I64Add,
                I32WrapI64,
                Call(push),
            ],
        );

        self.define_native_word(
            "UD/MOD",
            vec![ValueType::I64, ValueType::I64],
            vec![
                Call(pop),
                I64ExtendSI32,
                SetLocal(3),
                Call(pop_d),
                TeeLocal(2),
                GetLocal(3),
                I64RemU,
                I32WrapI64,
                Call(push),
                GetLocal(2),
                GetLocal(3),
                I64DivU,
                Call(push_d),
            ],
        );

        self.define_native_word(
            "MIN",
            vec![],
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
            vec![],
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
        self.define_native_word(
            "DMIN",
            vec![ValueType::I64, ValueType::I64],
            vec![
                Call(pop_d),
                TeeLocal(2),
                Call(pop_d),
                TeeLocal(3),
                GetLocal(2),
                GetLocal(3),
                I64LtS,
                Select,
                Call(push_d),
            ],
        );
        self.define_native_word(
            "DMAX",
            vec![ValueType::I64, ValueType::I64],
            vec![
                Call(pop_d),
                TeeLocal(2),
                Call(pop_d),
                TeeLocal(3),
                GetLocal(2),
                GetLocal(3),
                I64GtS,
                Select,
                Call(push_d),
            ],
        );

        self.define_native_word(
            "1+",
            vec![],
            vec![Call(pop), I32Const(1), I32Add, Call(push)],
        );
        self.define_native_word(
            "1-",
            vec![],
            vec![Call(pop), I32Const(1), I32Sub, Call(push)],
        );
        self.define_native_word(
            "INVERT",
            vec![],
            vec![Call(pop), I32Const(-1), I32Xor, Call(push)],
        );

        self.define_native_word("AND", vec![], binary_i32(I32And));
        self.define_native_word("OR", vec![], binary_i32(I32Or));
        self.define_native_word("XOR", vec![], binary_i32(I32Xor));
        self.define_native_word("LSHIFT", vec![], binary_i32(I32Shl));
        self.define_native_word("RSHIFT", vec![], binary_i32(I32ShrU));
        self.define_native_word("ARSHIFT", vec![], binary_i32(I32ShrS));

        // ( mask addr -- )
        let bitmanip = |manip: Vec<Instruction>| {
            let mut instructions = vec![
                Call(pop),
                TeeLocal(0),
                GetLocal(0),
                I32Load(2, 0),
                Call(pop),
            ];
            instructions.extend_from_slice(&manip);
            instructions.push(I32Store(2, 0));
            instructions
        };
        self.define_native_word("CSET", vec![], bitmanip(vec![I32Or]));
        self.define_native_word(
            "CRESET",
            vec![],
            bitmanip(vec![I32Const(-1), I32Xor, I32And]),
        );
        self.define_native_word("CTOGGLE", vec![], bitmanip(vec![I32Xor]));

        self.define_native_word(
            "2*",
            vec![],
            vec![Call(pop), I32Const(1), I32Shl, Call(push)],
        );
        self.define_native_word(
            "D2*",
            vec![],
            vec![Call(pop_d), I64Const(1), I64Shl, Call(push_d)],
        );
        self.define_native_word(
            "2/",
            vec![],
            vec![Call(pop), I32Const(1), I32ShrS, Call(push)],
        );
        self.define_native_word(
            "D2/",
            vec![],
            vec![Call(pop_d), I64Const(1), I64ShrS, Call(push_d)],
        );

        self.define_constant_word("FALSE", 0);
        self.define_constant_word("TRUE", -1);

        self.define_native_word("=", vec![], binary_i32_bool(I32Eq));
        self.define_native_word("D=", vec![ValueType::I64], binary_i64_bool(I64Eq));
        self.define_native_word("<>", vec![], binary_i32_bool(I32Ne));
        self.define_native_word("D<>", vec![ValueType::I64], binary_i64_bool(I64Ne));
        self.define_native_word("<", vec![], binary_i32_bool(I32LtS));
        self.define_native_word("U<", vec![], binary_i32_bool(I32LtU));
        self.define_native_word("D<", vec![ValueType::I64], binary_i64_bool(I64LtS));
        self.define_native_word(">", vec![], binary_i32_bool(I32GtS));
        self.define_native_word("U>", vec![], binary_i32_bool(I32GtU));
        self.define_native_word("D>", vec![ValueType::I64], binary_i64_bool(I64GtS));
        self.define_native_word("<=", vec![], binary_i32_bool(I32LeS));
        self.define_native_word("U<=", vec![], binary_i32_bool(I32LeU));
        self.define_native_word("D<=", vec![ValueType::I64], binary_i64_bool(I64LeS));
        self.define_native_word(">=", vec![], binary_i32_bool(I32GeS));
        self.define_native_word("U>=", vec![], binary_i32_bool(I32GeU));
        self.define_native_word("D>=", vec![ValueType::I64], binary_i64_bool(I64GeS));
        self.define_native_word(
            "=0",
            vec![],
            vec![I32Const(0), Call(pop), I32Eqz, I32Sub, Call(push)],
        );
        self.define_native_word(
            "<>0",
            vec![],
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
            vec![],
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
            vec![],
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
            vec![ColonValue::XT("EXECUTE"), ColonValue::XT("BYE")],
        );

        // Now that we're done adding things to the dictionary,
        // set values for CP (a var containing the next address in the dictionary)
        // and LATEST (a var containing the address of the final word).

        let cp_storage_address = self.get_execution_token("CP") + 4;
        let cp_bytes: Vec<u8> = self.cp.to_le_bytes().iter().copied().collect();
        self.assembler.add_data(cp_storage_address, cp_bytes);

        let latest_storage_address = self.get_execution_token("LATEST") + 4;
        let latest_bytes = self.latest_address.to_le_bytes().iter().copied().collect();
        self.assembler
            .add_data(latest_storage_address, latest_bytes);

        let run_xt = self.get_execution_token("RUN-WORD");
        #[cfg(test)]
        let xts = {
            // For testing, export every word as a function-which-EXECUTEs-that-word
            self.execution_tokens.clone()
        };
        #[cfg(not(test))]
        let xts = {
            // Export _start, the conventional WASI entry point
            let mut xts = HashMap::new();
            xts.insert("_start".to_owned(), self.get_execution_token("_start"));
            xts
        };
        for (word, xt) in xts {
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

    fn define_native_word(
        &mut self,
        name: &str,
        locals: Vec<ValueType>,
        instructions: Vec<Instruction>,
    ) {
        let code = self.create_native_callable(locals, instructions);
        self.define_word(name, code, &[]);
    }

    fn create_native_callable(
        &mut self,
        locals: Vec<ValueType>,
        mut instructions: Vec<Instruction>,
    ) -> u32 {
        instructions.push(End);
        let func = self.assembler.add_native_func(
            vec![ValueType::I32, ValueType::I32],
            vec![],
            locals,
            instructions,
        );
        self.assembler.add_table_entry(func)
    }

    fn get_execution_token(&self, name: &str) -> i32 {
        match self.execution_tokens.get(name) {
            Some(xt) => *xt,
            None => panic!("Could not find definition for \"{}\"", name),
        }
    }

    fn define_word(&mut self, name: &str, code: u32, parameter: &[u8]) {
        let old_latest_address = self.latest_address;
        let latest_address = self.cp;

        let mut data = Vec::with_capacity(header_size(name) as usize + parameter.len());
        data.push(name.len() as u8);
        data.extend_from_slice(name.as_bytes());
        data.extend_from_slice(&vec![0; required_padding(data.len() as i32) as usize]);
        data.extend_from_slice(&old_latest_address.to_le_bytes());
        data.extend_from_slice(&code.to_le_bytes());
        data.extend_from_slice(parameter);

        // for testing purposes, store execution tokens for later
        self.execution_tokens
            .insert(name.to_owned(), latest_address + header_size(name) - 4);

        let cp = self.cp + data.len() as i32;
        self.assembler.add_data(self.cp, data);
        self.cp = cp;
        self.latest_address = latest_address;
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
            push_d: 0,
            pop_d: 0,
            push_r: 0,
            pop_r: 0,
            docon: 0,
            dovar: 0,
            docol: 0,
            start: 0,
            ip: 0,
            cp: DICTIONARY_BASE,
            latest_address: 0,
            execution_tokens: HashMap::new(),
        }
        .initialize()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use wasmer::{imports, Function, ImportObject, Module, Store};

    use super::{ColonValue::*, Compiler};
    use crate::{compiler::ParamType, runtime::BasicRuntime};

    fn build<T>(func: T) -> Result<BasicRuntime>
    where
        T: FnOnce(&mut Compiler),
    {
        build_with_imports(func, |_, _| imports! {})
    }

    fn build_with_imports<T, F>(func: T, imports: F) -> Result<BasicRuntime>
    where
        T: FnOnce(&mut Compiler),
        F: FnOnce(&Store, &Module) -> ImportObject,
    {
        let mut compiler = Compiler::default();
        func(&mut compiler);
        let binary = compiler.compile()?;
        BasicRuntime::new(&binary, imports)
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
    fn should_handle_signed_div_mod() {
        let runtime = build(|_| {}).unwrap();
        type TestCase = ((i32, i32), (i32, i32));

        let test_cases: Vec<TestCase> = vec![
            ((1, 10), (0, 1)),
            ((7, 4), (1, 3)),
            ((-7, 4), (-2, 1)),
            ((7, -4), (-2, -1)),
            ((-7, -4), (1, -3)),
            ((-2, 1), (-2, 0)),
            ((-2, 2), (-1, 0)),
            ((-2, 3), (-1, 1)),
            ((-2, 4), (-1, 2)),
            ((8, -8), (-1, 0)),
        ];

        let results: Vec<TestCase> = test_cases
            .iter()
            .map(|case| {
                let ((divisor, dividend), _) = *case;

                runtime.push(divisor).unwrap();
                runtime.push(dividend).unwrap();
                runtime.execute("/MOD").unwrap();
                let quotient = runtime.pop().unwrap();
                let modulo = runtime.pop().unwrap();

                ((divisor, dividend), (quotient, modulo))
            })
            .collect();
        assert_eq!(results, test_cases);
    }

    #[test]
    fn should_handle_signed_div_rem() {
        let runtime = build(|_| {}).unwrap();
        type TestCase = ((i64, i32), (i32, i32));

        let test_cases: Vec<TestCase> = vec![
            ((7, 4), (1, 3)),
            ((-7, 4), (-1, -3)),
            ((7, -4), (-1, 3)),
            ((-7, -4), (1, -3)),
        ];

        let results: Vec<TestCase> = test_cases
            .iter()
            .map(|case| {
                let ((divisor, dividend), _) = *case;

                runtime.push_double(divisor).unwrap();
                runtime.push(dividend).unwrap();
                runtime.execute("SM/REM").unwrap();
                let quotient = runtime.pop().unwrap();
                let remainder = runtime.pop().unwrap();

                ((divisor, dividend), (quotient, remainder))
            })
            .collect();
        assert_eq!(results, test_cases);
    }

    #[test]
    fn should_handle_double_division() {
        let runtime = build(|_| {}).unwrap();
        runtime.push_double(123).unwrap();
        assert_eq!(0, runtime.pop().unwrap());
        assert_eq!(123, runtime.pop().unwrap());

        runtime.push(123).unwrap();
        runtime.push(0).unwrap();
        runtime.push(10).unwrap();
        runtime.execute("UD/MOD").unwrap();

        assert_eq!(runtime.pop_double().unwrap(), 12);
        assert_eq!(runtime.pop().unwrap(), 3);
    }

    #[test]
    fn should_compute_absolute_value() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(123).unwrap();
        runtime.execute("ABS").unwrap();
        assert_eq!(runtime.pop().unwrap(), 123);

        runtime.push(-123).unwrap();
        runtime.execute("ABS").unwrap();
        assert_eq!(runtime.pop().unwrap(), 123);
    }

    #[test]
    fn should_support_min_and_max() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.execute("MIN").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.execute("MAX").unwrap();
        assert_eq!(runtime.pop().unwrap(), 2);
    }

    #[test]
    fn should_support_bit_manipulations() {
        let runtime = build(|compiler| {
            compiler.define_variable_word("TESTBITS", 0x30);
        })
        .unwrap();

        runtime.push(0x0f).unwrap();
        runtime.execute("TESTBITS").unwrap();
        runtime.execute("CSET").unwrap();
        runtime.execute("TESTBITS").unwrap();
        runtime.execute("@").unwrap();
        assert_eq!(runtime.pop().unwrap(), 0x3f);

        runtime.push(0x03).unwrap();
        runtime.execute("TESTBITS").unwrap();
        runtime.execute("CRESET").unwrap();
        runtime.execute("TESTBITS").unwrap();
        runtime.execute("@").unwrap();
        assert_eq!(runtime.pop().unwrap(), 0x3c);

        runtime.push(0xcc).unwrap();
        runtime.execute("TESTBITS").unwrap();
        runtime.execute("CTOGGLE").unwrap();
        runtime.execute("TESTBITS").unwrap();
        runtime.execute("@").unwrap();
        assert_eq!(runtime.pop().unwrap(), 0xf0);
    }

    #[test]
    fn should_support_double_math() {
        let runtime = build(|_| {}).unwrap();
        runtime.push_double(420).unwrap();
        runtime.push_double(69).unwrap();
        runtime.execute("D+").unwrap();
        assert_eq!(runtime.pop_double().unwrap(), 489);
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
    fn should_conditionally_dup() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1337).unwrap(); // sentinel value at bottom
        runtime.push(1).unwrap();
        runtime.execute("?DUP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 1337);

        runtime.push(1337).unwrap(); // sentinel value at bottom
        runtime.push(0).unwrap();
        runtime.execute("?DUP").unwrap();
        assert_eq!(runtime.pop().unwrap(), 0);
        assert_eq!(runtime.pop().unwrap(), 1337);
    }

    #[test]
    fn should_over() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(1).unwrap();
        runtime.push(2).unwrap();
        runtime.execute("OVER").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);
        assert_eq!(runtime.pop().unwrap(), 2);
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
    fn should_pick() {
        let runtime = build(|_| {}).unwrap();

        runtime.push(10).unwrap();
        runtime.push(20).unwrap();
        runtime.push(30).unwrap();

        runtime.push(0).unwrap();
        runtime.execute("PICK").unwrap();
        assert_eq!(runtime.pop().unwrap(), 30);

        runtime.push(1).unwrap();
        runtime.execute("PICK").unwrap();
        assert_eq!(runtime.pop().unwrap(), 20);

        runtime.push(2).unwrap();
        runtime.execute("PICK").unwrap();
        assert_eq!(runtime.pop().unwrap(), 10);
    }

    #[test]
    fn should_report_depth() {
        let runtime = build(|_| {}).unwrap();

        runtime.execute("DEPTH").unwrap();
        assert_eq!(runtime.pop().unwrap(), 0);

        runtime.push(1).unwrap();
        runtime.execute("DEPTH").unwrap();
        assert_eq!(runtime.pop().unwrap(), 1);

        runtime.push(2).unwrap();
        runtime.execute("DEPTH").unwrap();
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
    fn should_support_string_literals() {
        let runtime = build(|compiler| {
            compiler.define_colon_word("SOME-WORD", vec![StringLit("Hello world!".to_owned())]);
        })
        .unwrap();

        runtime.execute("SOME-WORD").unwrap();
        assert_eq!(runtime.pop_string().unwrap(), "Hello world!");
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
                compiler.define_imported_word("SEVENTEEN", "test", "seventeen", vec![], vec![ParamType::I32, ParamType::I32]);
                compiler.define_imported_word("SWALLOW", "test", "swallow", vec![ParamType::I32, ParamType:: I32], vec![]);
                compiler.define_imported_word("TRIM", "test", "trim", vec![ParamType::I32, ParamType::I32], vec![ParamType::I32, ParamType::I32]);
                compiler.define_imported_word("HAS64", "test", "has64", vec![ParamType::I64], vec![ParamType::I64]);
            },
            |store, _| {
                imports! {
                    "test" => {
                        "seventeen" => Function::new_native(store, || (10, 7)),
                        "swallow" => Function::new_native(store, |_: i32, _: i32| {}),
                        "trim" => Function::new_native(store, |a: i32, b: i32| {
                            (a + 4, b - 8)
                        }),
                        "has64" => Function::new_native(store, |a: i64| { assert_eq!(a, 13); 64 as i64 }),
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

        runtime.push_double(13).unwrap();
        runtime.execute("HAS64").unwrap();
        assert_eq!(runtime.pop_double().unwrap(), 64);
    }
}
