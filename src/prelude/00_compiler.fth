-1 PARSE \ Define ' to make manual compilation easier
-1 PARSE \ ' DUP puts the XT of the word DUP on the stack. v useful for compilation
-1 PARSE \ Manually compiling : (') PARSE-NAME FIND-NAME DUP =0 IF -2 THROW THEN ;
-1 PARSE \ and : ' (') NAME>XT ;
2DROP 2DROP 2DROP 2DROP

CP @
3 C,
40 C,
39 C,
41 C,
0 CP +!
LATEST @ ,
LATEST !
(DOCOL) ,
PARSE-NAME PARSE-NAME FIND-NAME NAME>XT ,
PARSE-NAME FIND-NAME FIND-NAME NAME>XT ,
PARSE-NAME DUP FIND-NAME NAME>XT ,
PARSE-NAME =0 FIND-NAME NAME>XT ,
PARSE-NAME ?BRANCH FIND-NAME NAME>XT ,
CP @ 0 ,
PARSE-NAME LIT FIND-NAME NAME>XT ,
-2 ,
PARSE-NAME THROW FIND-NAME NAME>XT ,
CP @ SWAP !
PARSE-NAME EXIT FIND-NAME NAME>XT ,

CP @
1 C,
39 C,
2 CP +!
LATEST @ ,
LATEST !
(DOCOL) ,
(') (') NAME>XT ,
(') NAME>XT NAME>XT ,
(') EXIT NAME>XT ,

-1 PARSE \ Now I can just write "' DUP ," to compile DUP into a def, without this verbose mess.
-1 PARSE \ Real comments sound useful, adding those next. Comments use the same trick I'm doing manually here;
-1 PARSE \ parse input until you find a nonexistent character, then throw out the string you've parsed.
2DROP 2DROP 2DROP

CP @
129 C,
92 C,
2 CP +!
LATEST @ ,
LATEST !
(DOCOL) ,
' LIT , -1 , ' PARSE , ' 2DROP ,
' EXIT ,

\ Now I can write comments like this!

\ I'll define a short helper word HERE to get the latest address of the stack
\ Heavily commenting it to make it clearer what's going on
CP @ \ hold onto the head of the dictionary for later
4 C, \ this word has a 4-character name. The word C, adds a single byte to the the dictionary.
72 C, 69 C, 82 C, 69 C, \ ascii "HERE"
3 CP +!   \ Manually adding padding here so addresses are 4-byte aligned internally
LATEST @ , \ Link to the word before this in the dict. The word , adds a cell (4 bytes) to the dictionary.
LATEST !   \ Update the dictionary now that ENOUGH of this word is defined to not break anything
(DOCOL) , \ Mark this as a colon definition. (DOCOL) is a native word that starts running the body of a "colon definition""
\ The actual "body" of the definition begins now!
' CP ,  \ Compile the execution token (XT) of "CP" into the definition. At interpretation time, CP will get run.
' @ ,   \ Same thing for "@". "CP" pushes a variable address onto the stack, "@" reads the var at that address.
' EXIT , \ Exit goes at the end of every colon definition. It returns control to the caller.
\ And that's it! we've got "HERE".

\ define ALIGN to ensure the CP is aligned, so I don't haev to do it manually
HERE
5 C,
65 C, 76 C, 73 C, 71 C, 78 C, \ ascii "ALIGN"
2 CP +!
LATEST @ ,
LATEST !
(DOCOL) ,
' HERE ,
' ALIGNED ,
' CP ,
' ! ,
' EXIT ,

\ inline comments sound nice too, I'll add those next
HERE
129 C,  \ This word is immediate (128) and has a 1-character name (+1).
40 C, \ ascii "("
ALIGN
LATEST @ ,
LATEST !
(DOCOL) ,
' LIT ,
41 , \ ascii ")". The LIT word will return this value at interpretation time.
' PARSE , \ Read from input (this file) until we find that character.
' 2DROP , \ PARSE returns a string, but we don't need it so we can throw it out
' EXIT ,

\ I'm tired of looking up ASCII values
\ Defining CREATE to add words to the dictionary, so I don't have to so often.
HERE
6 C,
67 C, 82 C, 69 C, 65 C, 84 C, 69 C, \ CREATE
ALIGN
LATEST @ ,
LATEST !
(DOCOL) ,
' CP , ' @ ,                \ Keep a pointer to the def's head on the stack
' PARSE-NAME ,              \ CREATE reads the name of a new definition from input
' DUP , ' C, ,              \ Save the length of the name in the dictionary
HERE                        \ This is the start of a loop. Pushing CP onto the stack to track where to jump back to later
' DUP , ' <>0 ,             \ If we're still parsing the word
' ?BRANCH , HERE 0 ,        \ start of a conditional, so we need a forward jump. Saving space for the address to jump to here
' SWAP , ' DUP , ' C@ , ' UPCHAR , ' C, ,     \ add another char to the defintion
' 1+ , ' SWAP , ' 1- ,      \ increment string addr, decrement length
' BRANCH , SWAP ,           \ Unconditionally branch back to the start of the loop
HERE SWAP !                 \ Fill in the target of the forward jump, now that we've reached it
\ Looping/conditionals will be a lot easier once we've got a compiler to handle branching
' ALIGN ,                   \ Make sure the dictionary head is aligned
' 2DROP ,                   \ Clear the parsed name from the stack
' LATEST , ' @ , ' , ,      \ Compile the pointer to the previous word
' LATEST , ' ! ,            \ update that LATEST pointer to include our new word
' (DOVAR) , ' , ,           \ and default to the behavior of a variable
' EXIT ,

\ Now it's even less wordy to define words!
\ Add a helper to set the XT of the currently-defined word
( xt -- )
CREATE XT,
(DOCOL) LATEST @ NAME>XT !
' LATEST , ' @ , ' NAME>XT , ' ! ,
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

\ Enough manual compilation! time to build colon definitions.
\ Define a helper to set the IMMEDIATE flag on the last-defined word.
\ IMMEDIATE words have behavior during compilation-mode; non-IMMEDIATE words are just baked into the current def.
\ We need IMMEDIATE words to be able to shut the compiler off.
CREATE IMMEDIATE
(DOCOL) XT,
' LIT , 128 ,
' LATEST , ' @ ,  ' +! ,
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

\ The word HIDE hides the current definition from FIND-NAME
CREATE HIDE
(DOCOL) XT, ]
  LATEST @ DUP C@ 32 OR SWAP C!
EXIT [

\ The word REVEAL undoes HIDE
CREATE REVEAL
(DOCOL) XT, ]
  LATEST @ DUP C@ 32 INVERT AND SWAP C!
EXIT [

\ The word : starts a colon definition (hence the name)
CREATE :
(DOCOL) XT, ]
  CREATE (DOCOL) XT, \ Start defining a colon definition
  HIDE \ mark the def as hidden
  ] \ Switch to compilation mode
EXIT [ 

\ The word ; ends a colon definition and switches back to interpretation.
\ Just to be cheeky, let's use it while we define it
CREATE ;
(DOCOL) XT, ]
  LIT EXIT ,  \ Add EXIT to the end of the current definition
  REVEAL      \ mark the def as no longer hidden
  [ ' [ ,     \ switch to interpretation mode (both the def of ";" and the def being compiled)
; IMMEDIATE   \ and call it to finish compiling it!


\ And we're done! We have colon words!
\ Add other compilation utilities

: ['] \ ['] DUP pushes the XT of dup onto the stack at runtime
  ' \ get the XT
  LIT LIT , , \ and compile in a literal for it
; IMMEDIATE

\ [ 6 ] literal pushes 6 onto the stack at runtime
: LITERAL ( n -- ) 
  ['] LIT , ,
; IMMEDIATE

\ POSTPONE parses a word, and compiles its compilation semantics into the current word
: POSTPONE ( "ccc" -- )
  (') DUP NAME>IMMEDIATE?
  ?BRANCH [ HERE 0 , ]                  \ if
  NAME>XT ,                             \ compile the XT into the def
  BRANCH [ HERE 0 , SWAP HERE SWAP ! ]  \ else
  ['] LIT , NAME>XT , ['] , ,           \ compile "compile the XT" into the def
  [ HERE SWAP ! ]                       \ then
; IMMEDIATE

\ throw in recursion.
: RECURSE
  LATEST @ NAME>XT ,
; IMMEDIATE

\ ">BODY" gives you the address of a word defined with CREATE
: >BODY ( xt -- addr ) 4 + ;

\ "DOES>" lets you customize the runtime behavior of words you CREATEd.
: DOES>
  POSTPONE LIT
  HERE 0 , \ leave a gap (and track the address) for the new callable we're compiling
  POSTPONE XT, \ use that as the XT of whichever word was just created
  POSTPONE EXIT \ compile-time word over, runtime word begins
  \ now HERE is at the address of the runtime word, so we can fill in that gap
  (DODOES) HERE 8 LSHIFT OR SWAP !
; IMMEDIATE
