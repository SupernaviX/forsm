\ >IN is the offset in SOURCE that we've currently parsed to
variable >in host-deferred

4 cells constant source-record-size
: source>buf    0 cells + ;
: source>len    1 cells + ;
: source>id     2 cells + ;
: source>in     3 cells + ;

\ build a stack of source records
create source-records source-record-size 9 * allot
variable 'source
: @source 'source @ ;

\ initialize an "stdin" source record at the bottom of the stack
source-records source-record-size 8 * + constant source0
source0 'source !
tib @ source0 source>buf !
#tib @ source0 source>len !
0 source0 source>id !
0 source0 source>in !

\ redefine tib and #tib to use this source record as source-of-truth
source0 source>buf constant tib
source0 source>len constant #tib

\ build a stack of source buffers as well
128 constant source-buffer-size
create source-buffers source-buffer-size 8 * allot
source-buffers source-buffer-size 8 * + constant source-buffer0
variable 'source-buffer
source-buffer0 'source-buffer !

: take-source-buffer ( -- buf )
  'source-buffer @ source-buffer-size -
  dup 'source-buffer !
;
: return-source-buffer ( -- )
  source-buffer-size 'source-buffer +!
;

: add-file-source ( fid -- )
  @source
  >in @ over source>in !
  source-record-size -
  dup 'source !
  take-source-buffer over source>buf !
  0 over source>len !
  0 over source>in !
  source>id !
;

: drop-source ( -- )
  @source
  dup source>id @ close-file throw
  return-source-buffer
  source-record-size +
  dup source>in @ >in !
  'source !
;

\ reset the current source to be stdin
: reset-source ( -- )
  begin @source source0 <>
  while drop-source
  repeat
  0 >in !
;

\ SOURCE-ID describes the current input source.
\ 0 is user input.
\ A positive number is a file descriptor.
: source-id @source source>id @ ; host-deferred

\ SOURCE returns the current address and length of the input buffer
: source ( -- c-addr u )
  @source dup source>buf @
  swap source>len @
; host-deferred

\ REFILL tries to pull more data into source,
\ and returns a flag saying whether the source is empty now
: refill ( -- ? )
  0 >in ! \ reset >IN
  @source source>buf @ 128 @source source>id @ ( c-addr u1 fid )
  read-line throw ( u2 more? )
  swap @source source>len ! \ write how much we read
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
