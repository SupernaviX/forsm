16 BASE !
20 parse-name Woo type 20 emit 20 parse-name hoo! type 0a emit
0a BASE !
110 EMIT 105 emit 99 emit 101 emit

-1 PARSE-NAME \ Define ' to make manual compilation easier
DROP DROP
-1 PARSE-NAME \ ' DUP puts the XT of the word DUP on the stack. v useful for compilation
DROP DROP
-1 PARSE-NAME \ Manually compiling : ' 32 PARSE-NAME FIND-NAME NAME>XT ;
DROP DROP
1 C,
39 C,
LAST-WORD @ ,
CP @ 6 - LAST-WORD !
(DOCOL) ,
32 PARSE-NAME LIT FIND-NAME NAME>XT ,
32 ,
32 PARSE-NAME PARSE-NAME FIND-NAME NAME>XT ,
32 PARSE-NAME FIND-NAME FIND-NAME NAME>XT ,
32 PARSE-NAME NAME>XT FIND-NAME NAME>XT ,
32 PARSE-NAME EXIT FIND-NAME NAME>XT ,

-1 PARSE-NAME \ Real comments sound useful, adding those next
DROP DROP
1 C,
92 C,
LAST-WORD @ ,
CP @ 6 - LAST-WORD !
(DOCOL) ,
' LIT , -1 , ' PARSE-NAME , ' DROP , ' DROP ,
' EXIT ,

\ Now I can write comments like this!

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
 ' DROP , ' DROP ,           \ Clear the parsed name from the stack
' LAST-WORD , ' @ , ' , ,   \ Compile the pointer to the previous word
' LAST-WORD , ' ! ,         \ And finally, update that LAST-WORD pointer to include our new word!
' EXIT ,

CREATE pants
(DOCON) ,
69 ,
