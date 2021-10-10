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
41 PARSE-NAME ( but this kind of comment sounds nice too, adding that now ) DROP DROP

\ I'll heavily comment the next definition to make it clearer what's going on
1 C,  \ The name of this word is 1 character long. The word C, adds a single byte to the end of the definition
40 C, \ This is the literal for (
LAST-WORD @ , \ Link to the word before this in the dict The word , adds a cell (4 bytes) to the end of the current definition.
CP @ 6 - LAST-WORD ! \ Update the var pointing to the most recently-defined word
(DOCOL) , \ Mark this as a colon definition. (DOCOL) is a native word that starts running the body of a "colon definition"
\ The actual "body" of the definition begins now!
' LIT , \ Add a literal value to the word. This compilex the execution token (XT) of LIT into the definition. At interpretation time, that gets run.
41 , \ the literal value of ASCII end paren ) . The LIT word will return this value at interpretation time.
' PARSE-WORD , \ Read from input (this file) until we find that character.
' DROP , ' DROP , \ PARSE-WORD returns a string, but we don't need it so we can throw it out
' EXIT , \ Finally, return from the colon definition.

( Now I can add inline comments! )
