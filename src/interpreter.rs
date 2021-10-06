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

    gen.define_variable_word("BASE", 10);

    // try to parse a digit ( c -- n -1 | 0 )
    #[rustfmt::skip]
    gen.define_colon_word(
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
    gen.define_colon_word(
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
            XT("DUP"), XT("C@"), XT("?DIGIT"), XT("INVERT"),
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

fn build_interpreter(gen: &mut Generator) {
    // for now, store errors in here
    gen.define_variable_word("ERROR", 0);
    gen.define_colon_word("ERROR!", vec![XT("ERROR"), XT("!")]);
    gen.define_colon_word("ERROR@", vec![XT("ERROR"), XT("@")]);

    // Case-insensitive string equality against a known-capital string
    // ( c-addr1 u1 C-ADDR U2 -- ? )
    #[rustfmt::skip]
    gen.define_colon_word(
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
    gen.define_colon_word("NAME>U", vec![XT("C@"), Lit(31), XT("AND")]);

    // Given a name token, get the name ( nt -- c-addr u )
    #[rustfmt::skip]
    gen.define_colon_word(
        "NAME>STRING",
        vec![
            XT("DUP"), XT("1+"), // start of the name is 1 byte after the head
            XT("SWAP"), XT("NAME>U"),
        ],
    );

    // given a name token, get the name token before it ( nt -- nt | 0 )
    #[rustfmt::skip]
    gen.define_colon_word(
        "NAME>BACKWORD",
        vec![
            XT("DUP"), XT("NAME>U"), // get word length
            XT("1+"), XT("+"), // backword is 1 + len bytes into the def
            XT("@"),
        ],
    );

    // given a name token, get the interpret semantics ( nt -- xt | 0 )
    #[rustfmt::skip]
    gen.define_colon_word(
        "NAME>INTERPRET",
        vec![
            XT("DUP"), XT("C@"), Lit(128), XT("AND"), XT("<>0"), // is this an immediate word?
            QBranch(16), XT("DROP"), XT("FALSE"), // if so, it doesn't have interpret semantics
            Branch(24), // else, find em
            XT("DUP"), XT("NAME>U"), XT("+"), Lit(5), XT("+"), // xt is 1 + len + 4 bytes in
        ],
    );

    // Find the address of some word ( c-addr u -- nt | 0 )
    #[rustfmt::skip]
    gen.define_colon_word(
        "FIND-NAME",
        vec![
            XT("LAST-WORD"), XT("@"), // start at the end of the dictionary

            // start of loop
            XT("DUP"), XT("=0"), // if we've found null
            QBranch(20), // give up
            XT("DROP"), XT("DROP"), XT("DROP"), // flush the stack
            XT("FALSE"), XT("EXIT"), // and exit with haste and falseness

            XT(">R"), XT("OVER"), XT("OVER"), // set up copies of c-addr and u
            XT("R@"), XT("NAME>STRING"), // and extract the name from the nt
            XT("STR-UPPER-EQ?"),// Are they equal?

            QBranch(24), // this IS it chief!
            XT("DROP"), XT("DROP"), // get rid of c-addr and u
            XT("R>"), XT("EXIT"), // return the address of the word
            Branch(8), // this ain't it chief
            XT("R>"), XT("NAME>BACKWORD"), // go to the previous def
            Branch(-108), // end of loop
        ],
    );

    // execute words in a loop until the input buffer empties ( -- )
    #[rustfmt::skip]
    gen.define_colon_word(
        "INTERPRET",
        vec![
            // start of loop
            Lit(32), XT("PARSE-NAME"), // parse a space-delimited word from the TIB 

            XT("DUP"), XT("=0"),
            QBranch(12), // if the word is 0-length, we're done!
            XT("DROP"), XT("DROP"), XT("EXIT"),

            XT("OVER"), XT("OVER"), XT("FIND-NAME"), // look it up in the dictionary
            XT("DUP"), XT("<>0"),

            QBranch(68), // if we found the word in the dictionary,
            XT("NIP"), XT("NIP"), // clean the name out of the stack, we're done with it
            XT("NAME>INTERPRET"), // get the interpret semantics
            XT("DUP"), XT("=0"),
            QBranch(28), // if the word has no interpret semantics, error and exit
            Lit(-1), XT("ERROR!"), XT("DROP"), XT("EXIT"),
            Branch(4), // else, run it!
            XT("EXECUTE"),

            Branch(32),// if we did not find the word,
            XT("DROP"), // clean stack of "xt"
            XT("?NUMBER"), // maybe it's a number?
            XT("=0"), // if so, leave the value on the stack 
            QBranch(16), // if not, error and exit
            Lit(-2), XT("ERROR!"), XT("EXIT"),

            Branch(-180), // end of loop
        ],
    );
}
