1 C,
39 C,
LAST-WORD @ ,
CP @ 6 - LAST-WORD !
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
PARSE-NAME NAME>XT FIND-NAME NAME>XT ,
PARSE-NAME EXIT FIND-NAME NAME>XT ,

129 C,
92 C,
LAST-WORD @ ,
CP @ 6 - LAST-WORD !
(DOCOL) ,
' LIT , -1 , ' PARSE , ' DROP , ' DROP ,
' EXIT ,

\ NOW we support comments!
