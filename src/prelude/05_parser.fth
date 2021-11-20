\ SOURCE-ID describes the current input source.
\ 0 is user input.
\ A positive number is a file descriptor.
0 constant source-id host-deferred

\ SOURCE returns the current address and length of the input buffer
: source ( -- c-addr u )
  tib @ #tib @
; host-deferred

\ >IN is the offset in SOURCE that we've currently parsed to
variable >in host-deferred

\ REFILL tries to pull more data into source,
\ and returns a flag saying whether the source is empty now
: refill ( -- ? )
  0 >in ! \ reset >IN
  tib @ tib-max 0 read-line throw \ read a line
  swap #tib ! \ write how much we read
; host-deferred

: parse-area ( -- c-addr u ) source >in @ /string ;
: parse-consume ( n -- ) >in +! ;

: parse ( c -- c-addr u )
  >r
  parse-area over swap  \ store the parse-area start to return
  r@ take-until -rot \ compute the length and hold onto it
  r> prefix-length ( c-addr u u-trailing )
  over + parse-consume
;

: parse-name ( -- c-addr u )
  parse-area bl prefix-length parse-consume \ eat leading spaces
  bl parse
;

\ Now we have a parse-name which reads from the proper source at all times.

\ Define some nice-to-have utilities 
\ get the ascii value of the next character
: char parse-name drop c@ ;
\ compile the ascii value of the next char into the current def
: [char] parse-name drop c@ postpone literal ; immediate

\ Redefine every word that called parse or parse-name (to use the non-bootstrapped versions of them)
: \ -1 parse 2drop ; immediate
: ( [char] ) parse 2drop ; immediate
: ' ( -- xt )
  parse-name find-name
  dup =0 if -2 throw then
  name>xt
;
: create parse-name header ;
: variable ( -- ) create 0 , ;
: constant ( val -- ) create (docon) xt, , ;
: : create (docol) xt, hide ] ;
