use super::compiler::{ColonValue::*, Compiler, ParamType::*};

/* Build a very basic INTERPRET word */
pub fn build(compiler: &mut Compiler) {
    build_error_handling(compiler);
    build_io(compiler);
    build_parser(compiler);
    build_interpreter(compiler);
}

fn build_error_handling(compiler: &mut Compiler) {
    compiler.define_imported_word(
        "PROC-EXIT",
        "wasi_snapshot_preview1",
        "proc_exit",
        vec![I32],
        vec![],
    );
    // exit the process if the error is nonzero ( err -- )
    compiler.define_colon_word(
        "THROW",
        vec![XT("?DUP"), QBranch(4), XT("PROC-EXIT")],
    );
}

fn build_io(compiler: &mut Compiler) {
    // read arguments into a buffer
    // ( >argv >argv-buf -- err )
    compiler.define_imported_word(
        "ARGS-GET",
        "wasi_snapshot_preview1",
        "args_get",
        vec![I32, I32],
        vec![I32],
    );
    // find the size of the buffer needed for ARGS-GET
    // ( >argc >argv-buf-size -- err )
    compiler.define_imported_word(
        "ARGS-SIZES-GET",
        "wasi_snapshot_preview1",
        "args_sizes_get",
        vec![I32, I32],
        vec![I32],
    );
    // read from an FD into a buffer
    // ( fid iovec-arr iovec-len >bytes-read -- err )
    compiler.define_imported_word(
        "FD-READ",
        "wasi_snapshot_preview1",
        "fd_read",
        vec![I32, I32, I32, I32],
        vec![I32],
    );
    // write from a buffer to an FD
    // ( fid ciovec-arr ciovec-len >bytes-written -- err )
    compiler.define_imported_word(
        "FD-WRITE",
        "wasi_snapshot_preview1",
        "fd_write",
        vec![I32, I32, I32, I32],
        vec![I32],
    );
    // open a file by (relative) path
    // ( fid dirflags path-addr path-u oflags drights-base drights-inheriting fdflags >fid -- err )
    compiler.define_imported_word(
        "PATH-OPEN",
        "wasi_snapshot_preview1",
        "path_open",
        vec![I32, I32, I32, I32, I32, I64, I64, I32, I32],
        vec![I32],
    );
    // close a file descriptor
    // ( fid -- err )
    compiler.define_imported_word(
        "FD-CLOSE",
        "wasi_snapshot_preview1",
        "fd_close",
        vec![I32],
        vec![I32],
    );
    // get stats of a given preopened directory ( fid >stat -- err )
    compiler.define_imported_word(
        "FD-PRESTAT-GET",
        "wasi_snapshot_preview1",
        "fd_prestat_get",
        vec![I32, I32],
        vec![I32],
    );
    // get the name of a given preopened directory ( fid >name size -- err )
    compiler.define_imported_word(
        "FD-PRESTAT-DIR-NAME",
        "wasi_snapshot_preview1",
        "fd_prestat_dir_name",
        vec![I32, I32, I32],
        vec![I32],
    );
    // get information about an open file descriptor ( fid >fdstat -- err )
    compiler.define_imported_word(
        "FD-FDSTAT-GET",
        "wasi_snapshot_preview1",
        "fd_fdstat_get",
        vec![I32, I32],
        vec![I32],
    );

    compiler.define_constant_word("INBUF", 0x100);
    compiler.define_variable_word(">INBUF", 0);
    compiler.define_variable_word("#INBUF", 0);

    // iovec/ciovec are variables, the constants are just their addresses
    compiler.define_constant_word("IOVEC", 0xf8);
    compiler.define_constant_word("CIOVEC", 0xf0);

    compiler.define_variable_word(">SOURCE-ID", 0);
    compiler.define_colon_word("SOURCE-ID", vec![XT(">SOURCE-ID"), XT("@")]);

    // read a chunk of stdin into the file buffer ( -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "LOAD-INPUT-CHUNK",
        vec![
            // Prepare the iovec to read 1024 bytes into stdinbuf
            XT("INBUF"), XT("IOVEC"), XT("!"),
            Lit(1024), XT("IOVEC"), Lit(4), XT("+"), XT("!"),
            // try to read 1024 bytes
            XT("SOURCE-ID"), XT("IOVEC"), Lit(1), XT("#INBUF"), XT("FD-READ"), XT("THROW"),
            // reset stdinbuf pointer
            Lit(0), XT(">INBUF"), XT("!"),
        ],
    );

    compiler.define_colon_word(
        "INBUF-EMPTY?",
        vec![XT(">INBUF"), XT("@"), XT("#INBUF"), XT("@"), XT("=")],
    );

    compiler.define_constant_word("EOF", -1);
    // ( -- c|EOF )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "READ-INPUT-CHAR",
        vec![
            XT("INBUF-EMPTY?"), QBranch(24),
            XT("LOAD-INPUT-CHUNK"), // read into the stdin buffer if we need to
            XT("INBUF-EMPTY?"), QBranch(8),
            XT("EOF"), XT("EXIT"), // If stdin is STILL empty after loading a chunk, it's really empty
            XT("INBUF"), XT(">INBUF"), XT("@"), XT("+"), XT("C@"), // return the first character from the buffer
            Lit(1), XT(">INBUF"), XT("+!"), // advance the buffer pointer
        ],
    );

    // is this character a line terminator? ( c -- ? )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "IS-TERM?",
        vec![
            XT("DUP"), Lit(13), XT("="),
            XT("SWAP"), Lit(10), XT("="), XT("OR"),
        ],
    );

    // ( c-addr u -- n )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "ACCEPT",
        vec![
            XT("DUP"), XT("=0"), QBranch(16), // if someone is not asking for chars
            XT("2DROP"), Lit(0), XT("EXIT"), // return early

            XT("DUP"), XT(">R"), // hold onto the original requested length for later

            // start of loop ( c-addr u )
            XT("READ-INPUT-CHAR"),
            XT("DUP"), XT("EOF"), XT("<>"), // while we haven't hit the end of the file
            XT("OVER"), XT("IS-TERM?"), XT("AND"), // and we're reading newlines 
            QBranch(12),
            XT("DROP"), // discard the character
            Branch(-48),

            // start of loop ( c-addr u c|eof )
            XT("DUP"), XT("EOF"), XT("<>"), // while we haven't hit EOF
            XT("OVER"), XT("IS-TERM?"), XT("=0"), XT("AND"), // and we haven't hit newlines
            QBranch(64),
            XT("ROT"), XT("SWAP"), XT("OVER"), XT("C!"), // write to the buffer
            XT("1+"), XT("SWAP"), XT("1-"), // increment the buffer
            XT("DUP"), QBranch(12), XT("READ-INPUT-CHAR"), Branch(4), XT("EOF"), // grab the next char (or EOF if we're done)
            Branch(-100),

            XT("DROP"), // drop the final newline/EOF we read
            XT("NIP"), XT("R>"), XT("SWAP"), XT("-"), // return the char count
        ],
    );

    // using a variable as a 1-byte buffer holding the character to EMIT
    compiler.define_variable_word("EMIT-BUFFER", 0);
    compiler.define_variable_word("BYTES-WRITTEN", 0);

    // ( c -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "EMIT",
        vec![
            XT("EMIT-BUFFER"), XT("!"), // store the character in a buffer
            XT("EMIT-BUFFER"), XT("CIOVEC"), XT("!"), // set up the ciovec
            Lit(1), XT("CIOVEC"), Lit(4), XT("+"), XT("!"),
            Lit(1), XT("CIOVEC"), Lit(1), XT("BYTES-WRITTEN"), XT("FD-WRITE"), XT("THROW"),
        ],
    );

    // ( c-addr u -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "TYPE",
        vec![
            // store the buffer in our ciovec
            XT("SWAP"), XT("CIOVEC"), XT("!"),
            // start of loop
            XT("?DUP"), XT(">0"), QBranch(88), // while we have bytes to write..
            // try to write U bytes to the file
            XT("DUP"), XT("CIOVEC"), Lit(4), XT("+"), XT("!"),
            Lit(1), XT("CIOVEC"), Lit(1), XT("BYTES-WRITTEN"), XT("FD-WRITE"), XT("THROW"),
            XT("BYTES-WRITTEN"), XT("@"),
            // however many bytes we write, inc the buffer by that much
            XT("DUP"), XT("CIOVEC"), XT("+!"),
            // subtract BYTES-WRITTEN from what's left to write
            XT("-"),
            // and start again
            Branch(-104),
            // we're done!
        ],
    );

    compiler.define_constant_word("INIT-DIR-FD", 4);
    compiler.define_variable_word(">FD", 0);

    // ( c-addr u fam -- fileid err )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "OPEN-FILE",
        vec![
            XT("DROP"), // ignore fam for now, just implementing reads
            XT("INIT-DIR-FD"), Lit(0), XT("2SWAP"),
            Lit(0), Lit(0x1fffffff), Lit(0), Lit(0x1fffffff), Lit(0), // give ourselves full rights
            Lit(0), XT(">FD"),
            XT("PATH-OPEN"), // finally actually call this function
            XT(">FD"), XT("@"), XT("SWAP"),
        ],
    );
}

fn build_parser(compiler: &mut Compiler) {
    compiler.define_variable_word(">IN", 0);

    compiler.define_constant_word("TIB", 0x10);
    compiler.define_constant_word("TIB-MAX", 0xc0);
    compiler.define_variable_word("#TIB", 0);

    // refill TIB from stdin, return whether stdin is empty
    // ( -- ? )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "REFILL",
        vec![
            Lit(0), XT(">IN"), XT("!"), // Reset >IN
            XT("TIB"), XT("TIB-MAX"), XT("ACCEPT"), // Read a line
            XT("DUP"), XT("#TIB"), XT("!"), // store the new length of TIB
            XT("<>0"), // return if it's nonzero
        ],
    );

    // current address and length of the input buffer ( -- c-addr u )
    compiler.define_colon_word("SOURCE", vec![XT("TIB"), XT("#TIB"), XT("@")]);

    // is there anything to parse ( -- ? )
    compiler.define_colon_word(
        "PARSING?",
        vec![XT(">IN"), XT("@"), XT("SOURCE"), XT("NIP"), XT("<>")],
    );

    // get the address of the head of the parse area ( -- addr )
    compiler.define_colon_word(
        "'IN",
        vec![XT(">IN"), XT("@"), XT("SOURCE"), XT("DROP"), XT("+")],
    );

    // get first character in the parse area ( -- c )
    compiler.define_colon_word("IN@", vec![XT("'IN"), XT("C@")]);

    // Increment the head of the parse area ( -- )
    compiler.define_colon_word("1+IN!", vec![Lit(1), XT(">IN"), XT("+!")]);

    // Parse from the input buffer until we see a delimiter ( c -- c-addr u )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "PARSE",
        vec![
            // store the current parse location on the stack, it's word start
            XT("'IN"), XT("SWAP"),
            // parse until we see char-to-ignore or we're done
            XT("PARSING?"),
            XT("OVER"), XT("IN@"), XT("<>"),
            XT("AND"), QBranch(12),
            XT("1+IN!"),
            Branch(-40),
            // store the current parse location on the stack, it's word end
            XT("'IN"), XT("SWAP"),
            // consume ending delimiters
            XT("PARSING?"),
            XT("OVER"), XT("IN@"), XT("="),
            XT("AND"), QBranch(12),
            XT("1+IN!"),
            Branch(-40),
            // Get char-to-ignore off the stack, turn word end into a length
            XT("DROP"), XT("OVER"), XT("-"),
        ]
    );

    // Parse a word from the input buffer ( -- c-addr u )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "PARSE-NAME",
        vec![
            // ignore leading spaces
            XT("PARSING?"),
            Lit(32), XT("IN@"), XT("="),
            XT("AND"), QBranch(12),
            XT("1+IN!"),
            Branch(-44),
            // we are at the head of our word! parse the rest normally
            Lit(32), XT("PARSE"),
        ],
    );

    // Capitalize a character ( c -- C )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "UPCHAR",
        vec![
            XT("DUP"), Lit(97 /* a */), XT(">="),
            XT("OVER"), Lit(122 /* z */), XT("<="), XT("AND"),
            QBranch(12),
            Lit(32), XT("-")
        ],
    );

    compiler.define_variable_word("BASE", 10);

    // try to parse a digit ( c -- n -1 | 0 )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "?DIGIT",
        vec![
            XT("UPCHAR"), // parse hex as uppercase

            XT("DUP"), Lit(48), XT(">="),
            XT("OVER"), Lit(57), XT("<="), XT("AND"),
            QBranch(20), // if [0-9]
            Lit(48), XT("-"), // subtract '0'

            Branch(76), // else
            XT("DUP"), Lit(65), XT(">="),
            XT("OVER"), Lit(90), XT("<="), XT("AND"),
            QBranch(12), // if [A-Z]
            Lit(55), XT("-"), // subtract 'A', add 10

            Branch(12), // else
            XT("DROP"), XT("FALSE"), XT("EXIT"), // not a digit
            // end if

            XT("DUP"), XT("BASE"), XT("@"), XT(">="),
            QBranch(16), // if this isn't a valid digit in the current base
            XT("DROP"), XT("FALSE"),
            Branch(4), // else
            XT("TRUE"),
        ],
    );

    // Try parsing a string as a number ( c-addr u -- n -1 | 0 )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "?NUMBER",
        vec![
            XT("DUP"), XT("=0"),
            QBranch(12), // if the string is empty, it's not a number
            XT("2DROP"), XT("FALSE"), XT("EXIT"),

            XT("OVER"), XT("C@"), Lit(45), XT("="), // does it start with -?
            XT("DUP"), XT(">R"), // store whether it does on the return stack
            QBranch(16), // if it does, skip past the -
            XT("1-"), XT("SWAP"), XT("1+"), XT("SWAP"),

            XT("DUP"), XT("=0"),
            QBranch(12), // if we're out of characters NOW it's also not a number
            XT("2DROP"), XT("FALSE"), XT("EXIT"),

            XT("OVER"), XT("+"), XT(">R"), // store our final str-address in the return stack
            Lit(0), // store our running summation on the stack
            XT("SWAP"),

            // start loop ( n c-addr )
            XT("DUP"), XT("C@"), XT("?DIGIT"), XT("=0"),
            QBranch(24), // if the next char is NOT a digit
            XT("R>"), XT("R>"), XT("2DROP"), XT("2DROP"), // clean the stack
            XT("FALSE"), XT("EXIT"), // and get outta here
            // end if

            XT("ROT"), XT("BASE"), XT("@"), XT("*"), XT("+"), XT("SWAP"), // add digit to running total
            XT("1+"), // increment address
            XT("DUP"), XT("R@"), XT("="), // if we're out of input,
            QBranch(-96), // back to start of loop

            XT("DROP"), // we're done with the input string
            XT("R>"), XT("DROP"), // we're done with the target string

            // negate it if we have to, add TRUE, and we're good
            XT("R>"), QBranch(16),
            Lit(0), XT("SWAP"), XT("-"),
            XT("TRUE")
        ],
    );
}

fn build_interpreter(compiler: &mut Compiler) {
    // Case-insensitive string equality against a known-capital string
    // ( c-addr1 u1 C-ADDR U2 -- ? )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "STR-UPPER-EQ?",
        vec![
            XT("ROT"), XT("SWAP"), // ( c-addr1 c-addr2 u1 u2 )
            XT("OVER"), XT("<>"), QBranch(16), // If lengths mismatch, return now
            XT("2DROP"), XT("DROP"), XT("FALSE"), XT("EXIT"),
            // then

            // stack is now ( c-addr1 c-addr2 u )
            // start of loop
            XT("?DUP"), QBranch(96), // if length is 0, break outta the loop

            XT(">R"), // push length into return stack
            XT("OVER"), XT("C@"), XT("UPCHAR"), XT("OVER"), XT("C@"), XT("<>"), // are chars not-equal?
            QBranch(28), // if
            XT("R>"), XT("2DROP"), XT("DROP"), //fix the stacks
            XT("FALSE"), XT("EXIT"), // return false
            Branch(24), // else
            XT("SWAP"), XT("1+"), XT("SWAP"), XT("1+"), // increment pointers
            XT("R>"), XT("1-"), // get the count out of the return stack and decremented
            // then

            Branch(-108), // end of loop

            XT("2DROP"), XT("TRUE"), // if we made it this far we win!
        ],
    );

    // Given an address, return the next aligned address ( addr -- addr )
    compiler.define_colon_word("ALIGNED", vec![Lit(3), XT("+"), Lit(-4), XT("AND")]);

    // Given a name token, get its length ( nt -- u )
    compiler.define_colon_word("NAME>U", vec![XT("C@"), Lit(31), XT("AND")]);

    // Given a name token, get the name ( nt -- c-addr u )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "NAME>STRING",
        vec![
            XT("DUP"), XT("1+"), // start of the name is 1 byte after the head
            XT("SWAP"), XT("NAME>U"),
        ],
    );

    // given a name token, get the name token before it ( nt -- nt | 0 )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "NAME>BACKWORD",
        vec![
            XT("DUP"), XT("NAME>U"), // get word length
            XT("1+"), XT("+"), // backword is 1 + len bytes into the def
            XT("ALIGNED"),
            XT("@"),
        ],
    );

    // given a name token, does that token point to an immediate word? ( nt -- ? )
    compiler.define_colon_word(
        "NAME>IMMEDIATE?",
        vec![XT("C@"), Lit(128), XT("AND"), XT("<>0")],
    );
    // given a name token, is that token trampolined to a host word? ( nt -- ? )
    compiler.define_colon_word(
        "NAME>TRAMPOLINED?",
        vec![XT("C@"), Lit(64), XT("AND"), XT("<>0")],
    );
    compiler.define_colon_word("+NAME>IMMEDIATE?", vec![Lit(128), XT("SWAP"), XT("CSET")]);
    compiler.define_colon_word("+NAME>TRAMPOLINED?", vec![Lit(64), XT("SWAP"), XT("CSET")]);
    // The "hidden bit" is the high bit of the first character of the identifier.
    // This is always safe to use (even words with a 0-length name will have padding here),
    // and automatically causes string equality checks for ASCII input to fail.
    compiler.define_colon_word("+NAME>HIDDEN?", vec![Lit(32768), XT("SWAP"), XT("CSET")]);
    compiler.define_colon_word("-NAME>HIDDEN?", vec![Lit(32768), XT("SWAP"), XT("CRESET")]);

    // given a name token, get the execution token ( nt -- xt )
    // xt is 1 + len + 4 bytes in to the definition, plus alignment
    compiler.define_colon_word(
        "NAME>XT",
        vec![
            XT("DUP"),
            XT("NAME>U"),
            XT("+"),
            Lit(5),
            XT("+"),
            XT("ALIGNED"),
        ],
    );

    // Find the address of some word ( c-addr u -- nt | 0 )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "FIND-NAME",
        vec![
            XT("LATEST"), XT("@"), // start at the end of the dictionary

            // start of loop
            XT("DUP"), XT("=0"), // if we've found null
            QBranch(16), // give up
            XT("2DROP"), XT("DROP"), // flush the stack
            XT("FALSE"), XT("EXIT"), // and exit with haste and falseness

            XT(">R"), XT("2DUP"), // set up copies of c-addr and u
            XT("R@"), XT("NAME>STRING"), // and extract the name from the nt
            XT("STR-UPPER-EQ?"),// Are they equal?

            QBranch(20), // this IS it chief!
            XT("2DROP"), // get rid of c-addr and u
            XT("R>"), XT("EXIT"), // return the address of the word
            Branch(8), // this ain't it chief
            XT("R>"), XT("NAME>BACKWORD"), // go to the previous def
            Branch(-96), // end of loop
        ],
    );

    compiler.define_variable_word("STATE", 0);
    compiler.define_colon_word("COMPILING?", vec![XT("STATE"), XT("@")]);

    // append a cell to the end of the dictionary( n -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        ",",
        vec![
            XT("CP"), XT("@"), XT("!"), // save value at end of dictionary
            Lit(4), XT("CP"), XT("+!"), // shift end of dictionary over
        ],
    );

    // append a byte to the end of the dictionary ( n -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "C,",
        vec![
            XT("CP"), XT("@"), XT("C!"), // save value at end of dictionary
            Lit(1), XT("CP"), XT("+!"), // shift end of dictionary over
        ],
    );

    // append a compiled literal to the end of the dictionary ( n -- )
    compiler.define_colon_word(
        "COMPILE-LITERAL",
        vec![XT("LIT"), XT("LIT"), XT(","), XT(",")],
    );

    // Perform interpretation semantics for a word ( nt -- )
    compiler.define_colon_word("INTERPRET-NAME", vec![XT("NAME>XT"), XT("EXECUTE")]);

    // Perform compilation semantics for a word ( nt -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "COMPILE-NAME",
        vec![
            XT("DUP"), XT("NAME>XT"), // get the word's XT
            XT("SWAP"), XT("NAME>IMMEDIATE?"), // is the word immediate?
            QBranch(12),   // if so,
            XT("EXECUTE"), // run it right away
            Branch(4),     // else,
            XT(","),       // bake it in
        ],
    );

    // execute words in a loop until the input buffer empties ( -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "INTERPRET",
        vec![
            // start of loop
            XT("PARSE-NAME"), // parse a space-delimited word from input

            XT("DUP"), XT("=0"),
            QBranch(8), // if the word is 0-length, we're done!
            XT("2DROP"), XT("EXIT"),

            XT("2DUP"), XT("FIND-NAME"), // look it up in the dictionary
            XT("?DUP"),

            QBranch(44), // if we found the word in the dictionary,
            XT("NIP"), XT("NIP"), // clean the name out of the stack, we're done with it
            XT("COMPILING?"),
            QBranch(12),
            XT("COMPILE-NAME"),
            Branch(4),
            XT("INTERPRET-NAME"),

            Branch(48), // if we did not find the word,
            XT("?NUMBER"), // maybe it's a number?
            QBranch(24),  // if so, either bake the value in or leave it on the stack
            XT("COMPILING?"),
            QBranch(4),
            XT("COMPILE-LITERAL"),
            Branch(12), // if not, error and exit
            Lit(-1), XT("THROW"),

            Branch(-148), // end of loop
        ],
    );

    // include a file by path ( c-addr u -- )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "INCLUDED",
        vec![
            XT("SOURCE-ID"), XT("THROW"),         // for now, can't load 2 files at once
            Lit(0), XT("OPEN-FILE"), XT("THROW"), // actually open the file
            XT(">SOURCE-ID"), XT("!"),            // switch to the FD

            // start of execution loop
            XT("REFILL"),
            QBranch(12),     // quit if we are done
            XT("INTERPRET"), // run code
            Branch(-24),     // Good! Now do it again

            XT("SOURCE-ID"), XT("FD-CLOSE"), XT("THROW"), // close the file
            Lit(0), XT(">SOURCE-ID"), XT("!"),            // reset source
        ],
    );

    let start_instructions: Vec<_> = std::fs::read_dir("./src/prelude")
        .unwrap()
        .flat_map(|file| {
            let raw_name = file.unwrap().file_name();
            let name = format!("src/prelude/{}", raw_name.to_string_lossy());
            vec![StringLit(name), XT("INCLUDED")]
        })
        .collect();

    compiler.define_colon_word("_start", start_instructions);
}
