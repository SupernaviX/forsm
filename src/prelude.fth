16 BASE !
20 parse-name Woo type 20 emit 20 parse-name hoo! type 0a emit
0a BASE !
110 EMIT 105 emit 99 emit 101 emit

-1 PARSE-NAME \ Define ' to make manual compilation easier
-1 PARSE-NAME \ ' DUP puts the XT of the word DUP on the stack. v useful for compilation
-1 PARSE-NAME \ Manually compiling : ' BL PARSE-NAME FIND-NAME DUP =0 IF -2 THROW THEN NAME>XT ;
DROP DROP DROP DROP DROP DROP
1 C,
39 C,
LAST-WORD @ ,
CP @ 6 - LAST-WORD !
(DOCOL) ,
32 PARSE-NAME LIT FIND-NAME NAME>XT ,
32 ,
32 PARSE-NAME PARSE-NAME FIND-NAME NAME>XT ,
32 PARSE-NAME FIND-NAME FIND-NAME NAME>XT ,
32 PARSE-NAME DUP FIND-NAME NAME>XT ,
32 PARSE-NAME =0 FIND-NAME NAME>XT ,
32 PARSE-NAME ?BRANCH FIND-NAME NAME>XT ,
CP @ 0 ,
32 PARSE-NAME LIT FIND-NAME NAME>XT ,
-2 ,
32 PARSE-NAME THROW FIND-NAME NAME>XT ,
CP @ SWAP !
32 PARSE-NAME NAME>XT FIND-NAME NAME>XT ,
32 PARSE-NAME EXIT FIND-NAME NAME>XT ,

-1 PARSE-NAME \ Now I can just write "' DUP ," to compile DUP into a def, without this verbose mess.
-1 PARSE-NAME \ Real comments sound useful, adding those next. Comments use the same trick I'm doing manually here;
-1 PARSE-NAME \ parse input until you find a nonexistent character, then throw out the string you've parsed.
DROP DROP DROP DROP DROP DROP

129 C,
92 C,
LAST-WORD @ ,
CP @ 6 - LAST-WORD !
(DOCOL) ,
' LIT , -1 , ' PARSE-NAME , ' DROP , ' DROP ,
' EXIT ,

\ Now I can write comments like this!
\ But inline comments sound nice too, I'll add those next

\ I'll heavily comment the next definition to make it clearer what's going on
129 C,  \ This word is immediate (128) and has a 1-character name (+1). The word C, adds a single byte to the end of the definition.
40 C, \ This is the literal for (
LAST-WORD @ , \ Link to the word before this in the dict The word , adds a cell (4 bytes) to the end of the current definition.
CP @ 6 - LAST-WORD ! \ Update the var pointing to the most recently-defined word
(DOCOL) , \ Mark this as a colon definition. (DOCOL) is a native word that starts running the body of a "colon definition"
\ The actual "body" of the definition begins now!
' LIT , \ Add a literal value to the word. This compilex the execution token (XT) of LIT into the definition. At interpretation time, that gets run.
41 , \ the literal value of ascii ) . The LIT word will return this value at interpretation time.
' PARSE-NAME , \ Read from input (this file) until we find that character.
' DROP , ' DROP , \ PARSE-NAME returns a string, but we don't need it so we can throw it out
' LIT , 32 , ' PARSE-NAME , ' DROP , ' DROP , \ and do the same to consume the next space-delimited word, which IS the )
' EXIT , \ Finally, return from the colon definition.

10 EMIT 67 EMIT 79 EMIT ( Now I can add inline comments! ) 79 EMIT 76 EMIT

\ I'm tired of looking up ASCII values and manually doing math on string lengths.
\ Defining CREATE to add words to the dictionary, so I don't have to so often.
6 C,
67 C, 82 C, 69 C, 65 C, 84 C, 69 C, \ CREATE
LAST-WORD @ ,
CP @ 11 - LAST-WORD !
(DOCOL) ,
' CP , ' @ ,                \ Keep a pointer to the def's head on the stack
' LIT , 32 , ' PARSE-NAME , \ CREATE reads the name of a new definition from input
' DUP , ' C, ,              \ Save the length of the name in the dictionary
CP @                        \ This is the start of a loop. Pushing CP onto the stack to track where to jump back to later
' DUP , ' <>0 ,             \ If we're still parsing the word
' ?BRANCH , CP @ 0 ,        \ start of a conditional, so we need a forward jump. Saving space for the address to jump to here
' SWAP , ' DUP , ' C@ , ' UPCHAR , ' C, ,     \ add another char to the defintion
' 1+ , ' SWAP , ' 1- ,      \ increment string addr, decrement length
' BRANCH , SWAP ,           \ Unconditionally branch back to the start of the loop
CP @ SWAP !                 \ Fill in the target of the forward jump, now that we've reached it
\ Looping/conditionals will be a lot easier once we've got a compiler to handle branching
 ' DROP , ' DROP ,          \ Clear the parsed name from the stack
' LAST-WORD , ' @ , ' , ,   \ Compile the pointer to the previous word
' LAST-WORD , ' ! ,         \ update that LAST-WORD pointer to include our new word
' (DOVAR) , ' , ,           \ and default to the behavior of a variable
' EXIT ,

\ Now it's even less wordy to define words!
\ Add a helper to set the XT of the currently-defined word
( xt -- )
CREATE XT,
(DOCOL) CP @ 4 - !
' LAST-WORD , ' @ , ' NAME>XT , ' ! ,
' EXIT ,

\ Support single-cell variables ( -- )
CREATE VARIABLE
(DOCOL) XT,
' CREATE , ' LIT , 0 , ' , , \ Just CREATE but also reserve a cell of memory
' EXIT ,

\ Support single-cell constants ( val -- )
CREATE CONSTANT
(DOCOL) XT,
' CREATE ,
' (DOCON) , ' XT, , \ Set the behavior of the new constant
' , , \ and just store the input param after it on the stack (as (DOCON) wants)
' EXIT ,

32 CONSTANT BL

\ Enough manual compilation! time to build colon definitions.
\ Define a helper to set the IMMEDIATE flag on the last-defined word.
\ IMMEDIATE words have behavior during compilation-mode; non-IMMEDIATE words are just baked into the current def.
\ We need IMMEDIATE words to be able to shut the compiler off.
CREATE IMMEDIATE
(DOCOL) XT,
' LIT , 128 ,
' LAST-WORD , ' @ ,  ' +! ,
' EXIT ,

\ The word ] starts compilation.
CREATE ]
(DOCOL) XT,
' LIT , -1 , ' STATE , ' ! ,
' EXIT ,

\ The word [ stops compilation, and goes back to interpreter mode.
CREATE [
(DOCOL) XT,
' LIT , 0 , ' STATE , ' ! ,
' EXIT ,
IMMEDIATE \ THIS has to be immediate, otherwise the compiler runs forever!

\ The word : starts a colon definition (hence the name)
CREATE :
(DOCOL) XT,
] CREATE (DOCOL) XT, [ \ Start defining a colon definition
' ] , \ Switch to compilation mode
' EXIT ,

\ The word ; ends a colon definition and switches back to interpretation
CREATE ;
(DOCOL) XT,
' LIT , ' EXIT , ' , , \ Add EXIT to the end of the current definition
' [ , \ Switch to interpretation mode
' EXIT ,
IMMEDIATE

\ And we're done! We have colon words!
\ now let's make some niceties.
: here ( -- n ) cp @ ;

\ like branching!
: >mark here 0 , ;
: >resolve here swap ! ;
: <mark here ;
: <resolve , ;

: ['] \ ['] DUP pushes the XT of dup onto the stack at runtime
  ' \ get the XT
  [ ' LIT , ' LIT , ] , \ compile LIT
  , \ compile the XT
  ; immediate

\ Conditionals!
: if ['] ?branch , >mark ; immediate
: else ['] branch , >mark swap >resolve ; immediate
: then >resolve ; immediate

\ POSTPONE parses a word, and compiles its compilation semantics into the current word
: POSTPONE ( "ccc" -- )
  bl parse-name find-name dup =0 if -1 throw then \ Find the nt for the next word, throw if we cna't
  dup name>immediate?
    if    name>xt , \ compile this XT into the def
    else  ['] lit , name>xt , ['] , , \ compile "compile this XT" into the def
    then
  ; immediate

: endif POSTPONE then ; immediate

: postplus POSTPONE + ; immediate

: test ( n ? -- n )
  if 2 + else 1 + endif 3 postplus ;

65 0 test emit
65 -1 test emit