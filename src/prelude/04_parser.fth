\ SOURCE returns the current address and length of the input buffer
' source
defer source ( -- addr u )
' source defer! \ while we bootstrap, use whatever source the host uses

\ >in is a variable defined in the host

: parse-area ( -- c-addr u ) source >in @ /string ;
: parse-consume ( n -- ) >in +! ;

: length-while ( c-addr1 u1 c -- n )
  over >r
  remove-start
  nip r> swap -
;
\ return new string and # of chars consumed
: take-until ( c-addr1 u1 c -- c-addr2 u2 n )
  over >r
  scan
  r> over -
;

: parse ( c -- c-addr u )
  >r
  parse-area over swap  \ store the parse-area start to return
  r@ take-until -rot \ compute the length and hold onto it
  r> length-while ( c-addr u u-trailing )
  over + parse-consume
;

: parse-name ( -- c-addr u )
  parse-area bl length-while parse-consume \ eat leading spaces
  bl parse
;

\ get the ascii value of the next character
: char parse-name drop c@ ;
\ compile the ascii value of the next char into the current def
: [char] parse-name drop c@ postpone literal ; immediate
