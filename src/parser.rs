use super::generator::{ColonValue::*, Generator};

pub fn build_parser(gen: &mut Generator) {
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
        "PARSE",
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
