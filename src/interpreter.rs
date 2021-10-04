use super::generator::{ColonValue::*, Generator};

/* Build a very basic INTERPRET word */
pub fn build(gen: &mut Generator) {
    build_parser(gen);
    build_interpreter(gen);
}

fn build_parser(gen: &mut Generator) {
    let buf_start = 0x100;
    gen.define_variable_word("TIB", buf_start);
    gen.define_variable_word("#TIB", 0);
    gen.define_variable_word(">IN", 0);

    // is there anything to parse( -- ? )
    gen.define_colon_word(
        "PARSING?",
        vec![XT(">IN"), XT("@"), XT("#TIB"), XT("@"), XT("<>")],
    );

    // get the address of the head of the parse area ( -- addr )
    gen.define_colon_word("'IN", vec![XT(">IN"), XT("@"), XT("TIB"), XT("@"), XT("+")]);

    // get first character in the parse area ( -- c )
    gen.define_colon_word("IN@", vec![XT("'IN"), XT("C@")]);

    // Increment the head of the parse area ( -- )
    gen.define_colon_word("1+IN!", vec![Lit(1), XT(">IN"), XT("+!")]);

    // Parse a word from the input buffer ( c -- c-addr u )
    #[rustfmt::skip]
    gen.define_colon_word(
        "PARSE-NAME",
        vec![
            // ignore leading chars-to-ignore
            XT("PARSING?"),
            XT("OVER"), XT("IN@"), XT("="),
            XT("AND"), QBranch(12),
            XT("1+IN!"),
            Branch(-40),
            // we are at the head of our word! get it on the stack
            XT("'IN"), XT("SWAP"),
            // keep parsing until we DO see chars-to-ignore or we're done
            XT("PARSING?"),
            XT("OVER"), XT("IN@"), XT("<>"),
            XT("AND"), QBranch(12),
            XT("1+IN!"),
            Branch(-40),
            // Get char-to-ignore off the stack, put len on instead
            XT("DROP"),
            XT("DUP"), XT("'IN"), XT("SWAP"), XT("-")
        ],
    );
}

fn build_interpreter(gen: &mut Generator) {
    // Capitalize a character ( c -- C )
    #[rustfmt::skip]
    gen.define_colon_word(
        "UPCHAR",
        vec![
            XT("DUP"), Lit(97 /* a */), XT(">="),
            XT("OVER"), Lit(122 /* z */), XT("<="), XT("AND"),
            QBranch(12),
            Lit(32), XT("-")
        ],
    );

    // Case-insensitive string equality against a known-capital string
    // ( c-addr1 u1 C-ADDR U2 -- ? )
    #[rustfmt::skip]
    gen.define_colon_word(
        "STR-UPPER-EQ",
        vec![
            XT("ROT"), XT("SWAP"), // ( c-addr1 c-addr2 u1 u2 )
            XT("OVER"), XT("<>"), QBranch(20), // If lengths mismatch, return now
            XT("DROP"), XT("DROP"), XT("DROP"), XT("FALSE"), XT("EXIT"),
            // then

            // stack is now ( c-addr1 c-addr2 u )
            // start of loop
            XT("DUP"), XT(">0"), QBranch(100), // if length is 0, break outta the loop

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
}
