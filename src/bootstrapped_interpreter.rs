use super::compiler::{ColonValue::*, Compiler};

/* Build a very basic INTERPRET word */
pub fn build(compiler: &mut Compiler) {
    // for now, store errors in here
    compiler.define_variable_word("ERROR", 0);
    #[rustfmt::skip]
    compiler.define_colon_word(
        "THROW",
        vec![
            // store it
            XT("DUP"), XT("ERROR"), XT("!"),
            // halt control flow iff it is nonzero
            QBranch(4), XT("STOP"),
        ],
    );
    compiler.define_colon_word("ERROR@", vec![XT("ERROR"), XT("@")]);

    build_io(compiler);
    build_parser(compiler);
    build_interpreter(compiler);
}

fn build_io(compiler: &mut Compiler) {
    compiler.define_variable_word(">IN", 0);

    // read from an FD into a vec
    // ( fid iovec-arr iovec-len >bytes-read -- err )
    compiler.define_imported_word("IO", "FD-READ", 4, 1);
    // output a single character to stdout ( c -- )
    compiler.define_imported_word("IO", "EMIT", 1, 0);
    // output a string to stdout ( c-addr u -- )
    compiler.define_imported_word("IO", "TYPE", 2, 0);

    compiler.define_variable_word("TIB", 0x10);
    compiler.define_constant_word("TIB-MAX", 0xc0);
    compiler.define_variable_word("#TIB", 0);

    // and we read this many bytes at a time from files
    compiler.define_constant_word("CHUNK-LEN", 1024);

    compiler.define_constant_word("STDINBUF", 0x100);
    compiler.define_variable_word(">STDINBUF", 0);
    compiler.define_variable_word("#STDINBUF", 0);

    // iovec is a variable, the constant is just its address
    compiler.define_constant_word("IOVEC", 0xf8);
    compiler.define_variable_word("IOVECS", 0xf8);
    compiler.define_variable_word("BYTES-READ", 0);

    // read all of stdin into the file buffer
    #[rustfmt::skip]
    compiler.define_colon_word(
        "LOAD-STDIN",
        vec![
            // Prepare the iovec to read 1024 bytes into stdinbuf
            XT("STDINBUF"), XT("IOVEC"), XT("!"),
            XT("CHUNK-LEN"), XT("IOVEC"), Lit(4), XT("+"), XT("!"),
            // start o' loop
            // try to read 1024 bytes
            Lit(0), XT("IOVECS"), Lit(1), XT("BYTES-READ"), XT("FD-READ"), XT("THROW"),
            XT("BYTES-READ"), XT("@"), XT("CHUNK-LEN"), XT("="),
            QBranch(20), // loop while we've read CHUNK-LEN bytes
            XT("CHUNK-LEN"), XT("IOVEC"), XT("+!"), // start reading the next chunk
            Branch(-76),
            // now just compute the length (address of final write + length of final write - initial address)
            XT("IOVEC"), XT("@"), XT("BYTES-READ"), XT("@"), XT("+"), XT("STDINBUF"), XT("-"),
            XT("#STDINBUF"), XT("!"),
            // oh and reset the STDINBUF pointer
            Lit(0), XT(">STDINBUF"), XT("!"),
        ],
    );

    compiler.define_colon_word(
        "STDIN-EMPTY?",
        vec![XT(">STDINBUF"), XT("@"), XT("#STDINBUF"), XT("@"), XT("=")],
    );
    compiler.define_colon_word(
        "'STDIN",
        vec![XT("STDINBUF"), XT(">STDINBUF"), XT("@"), XT("+")],
    );
    compiler.define_colon_word("STDIN@", vec![XT("'STDIN"), XT("C@")]);

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
            // if the stdin STDINBUF is empty, reload it
            XT("STDIN-EMPTY?"), QBranch(4), XT("LOAD-STDIN"),

            XT("STDIN-EMPTY?"), XT("=0"), // while there's anything to parse
            XT("STDIN@"), XT("IS-TERM?"), XT("AND"), // and this starts with a lineterm,
            QBranch(24),
            Lit(1), XT(">STDINBUF"), XT("+!"), // skip it
            Branch(-52),

            XT("DUP"), XT(">R"), // hold onto the original requested length for later

            XT("DUP"), XT(">0"), // while the caller wants more
            XT("STDIN-EMPTY?"), XT("=0"), XT("AND"), // and there's something to parse
            XT("STDIN@"), XT("IS-TERM?"), XT("=0"), XT("AND"), // and it is not a lineterm
            QBranch(52),
            XT("SWAP"), XT("STDIN@"), XT("OVER"), XT("C!"), // copy the char to the buffer
            XT("1+"), XT("SWAP"), XT("1-"), // keep incrementing the buffer
            Lit(1), XT(">STDINBUF"), XT("+!"), // consume the char
            Branch(-96),

            XT("NIP"), XT("R>"), XT("SWAP"), XT("-"), // return the char count
        ],
    );

    // refill TIB from stdin, return whether stdin is empty
    // ( -- ? )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "REFILL",
        vec![
            Lit(0), XT(">IN"), XT("!"), // Reset >IN
            XT("TIB"), XT("@"), XT("TIB-MAX"), XT("ACCEPT"), // Read a line
            XT("DUP"), XT("#TIB"), XT("!"), // store the new length of TIB
            XT("<>0"), // return if it's nonzero
        ],
    );
}

fn build_parser(compiler: &mut Compiler) {
    // current address and length of the input buffer ( -- c-addr u )
    compiler.define_colon_word("SOURCE", vec![XT("TIB"), XT("@"), XT("#TIB"), XT("@")]);

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
            // Get char-to-ignore off the stack, put len on instead
            XT("DROP"),
            XT("DUP"), XT("'IN"), XT("SWAP"), XT("-")
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
            QBranch(16), // if the string is empty, it's not a number
            XT("DROP"), XT("DROP"), XT("FALSE"), XT("EXIT"),

            XT("OVER"), XT("C@"), Lit(45), XT("="), // does it start with -?
            XT("DUP"), XT(">R"), // store whether it does on the return stack
            QBranch(16), // if it does, skip past the -
            XT("1-"), XT("SWAP"), XT("1+"), XT("SWAP"),

            XT("DUP"), XT("=0"),
            QBranch(16), // if we're out of characters NOW it's also not a number
            XT("DROP"), XT("DROP"), XT("FALSE"), XT("EXIT"),

            XT("OVER"), XT("+"), XT(">R"), // store our final str-address in the return stack
            Lit(0), // store our running summation on the stack
            XT("SWAP"),

            // start loop ( n c-addr )
            XT("DUP"), XT("C@"), XT("?DIGIT"), XT("=0"),
            QBranch(32), // if the next char is NOT a digit
            XT("R>"), XT("R>"), XT("DROP"), XT("DROP"), XT("DROP"), XT("DROP"), // clean the stack
            XT("FALSE"), XT("EXIT"), // and get outta here
            // end if

            XT("ROT"), XT("BASE"), XT("@"), XT("*"), XT("+"), XT("SWAP"), // add digit to running total
            XT("1+"), // increment address
            XT("DUP"), XT("R@"), XT("="), // if we're out of input,
            QBranch(-104), // back to start of loop

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
            XT("OVER"), XT("<>"), QBranch(20), // If lengths mismatch, return now
            XT("DROP"), XT("DROP"), XT("DROP"), XT("FALSE"), XT("EXIT"),
            // then

            // stack is now ( c-addr1 c-addr2 u )
            // start of loop
            XT("DUP"), XT("<>0"), QBranch(100), // if length is 0, break outta the loop

            XT(">R"), // push length into return stack
            XT("OVER"), XT("C@"), XT("UPCHAR"), XT("OVER"), XT("C@"), XT("<>"), // are chars not-equal?
            QBranch(32), // if
            XT("R>"), XT("DROP"), XT("DROP"), XT("DROP"), //fix the stacks
            XT("FALSE"), XT("EXIT"), // return false
            Branch(24), // else
            XT("SWAP"), XT("1+"), XT("SWAP"), XT("1+"), // increment pointers
            XT("R>"), XT("1-"), // get the count out of the return stack and decremented
            // then

            Branch(-116), // end of loop

            XT("DROP"), XT("DROP"), XT("DROP"), XT("TRUE"), // if we made it this far we win!
        ],
    );

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
            XT("@"),
        ],
    );

    // given a name token, does that token point to an immedaite word? ( nt -- ? )
    compiler.define_colon_word(
        "NAME>IMMEDIATE?",
        vec![XT("C@"), Lit(128), XT("AND"), XT("<>0")],
    );

    // given a name token, get the execution token ( nt -- xt )
    // xt is 1 + len + 4 bytes in to the definition
    compiler.define_colon_word(
        "NAME>XT",
        vec![XT("DUP"), XT("NAME>U"), XT("+"), Lit(5), XT("+")],
    );

    // Find the address of some word ( c-addr u -- nt | 0 )
    #[rustfmt::skip]
    compiler.define_colon_word(
        "FIND-NAME",
        vec![
            XT("LATEST"), XT("@"), // start at the end of the dictionary

            // start of loop
            XT("DUP"), XT("=0"), // if we've found null
            QBranch(20), // give up
            XT("DROP"), XT("DROP"), XT("DROP"), // flush the stack
            XT("FALSE"), XT("EXIT"), // and exit with haste and falseness

            XT(">R"), XT("OVER"), XT("OVER"), // set up copies of c-addr and u
            XT("R@"), XT("NAME>STRING"), // and extract the name from the nt
            XT("STR-UPPER-EQ?"),// Are they equal?
            XT("R@"), XT("@"), Lit(32), XT("AND"), XT("=0"), XT("AND"), // AND is the word not hidden?

            QBranch(24), // this IS it chief!
            XT("DROP"), XT("DROP"), // get rid of c-addr and u
            XT("R>"), XT("EXIT"), // return the address of the word
            Branch(8), // this ain't it chief
            XT("R>"), XT("NAME>BACKWORD"), // go to the previous def
            Branch(-136), // end of loop
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
            QBranch(12), // if the word is 0-length, we're done!
            XT("DROP"), XT("DROP"), XT("EXIT"),

            XT("OVER"), XT("OVER"), XT("FIND-NAME"), // look it up in the dictionary
            XT("DUP"), XT("<>0"),

            QBranch(44), // if we found the word in the dictionary,
            XT("NIP"), XT("NIP"), // clean the name out of the stack, we're done with it
            XT("COMPILING?"),
            QBranch(12),
            XT("COMPILE-NAME"),
            Branch(4),
            XT("INTERPRET-NAME"),

            Branch(52), // if we did not find the word,
            XT("DROP"), // clean stack of "xt"
            XT("?NUMBER"), // maybe it's a number?
            QBranch(24),  // if so, either bake the value in or leave it on the stack
            XT("COMPILING?"),
            QBranch(4),
            XT("COMPILE-LITERAL"),
            Branch(12), // if not, error and exit
            Lit(-1), XT("THROW"),

            Branch(-164), // end of loop
        ],
    );

    // Very basic main loop
    compiler.define_colon_word(
        "QUIT",
        vec![
            XT("REFILL"),
            QBranch(12),     // quit if we are done
            XT("INTERPRET"), // run code
            Branch(-24),     // Good! Now do it again
            XT("STOP"),      // stop execution when we are done
        ],
    )
}
