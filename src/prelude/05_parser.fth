\ >IN is the offset in SOURCE that we've currently parsed to
variable >in host-deferred

6 cells constant |source|
: source.buf    0 cells + ;
: source.len    1 cells + ;
: source.id     2 cells + ;
: source.in     3 cells + ;
: source.name   4 cells + ;
: source.name#  5 cells + ;

\ build a stack of source records
create source-records |source| 9 * allot
variable 'source
: @source 'source @ ;

\ initialize an "stdin" source record at the bottom of the stack
source-records |source| 8 * + constant source0
source0 'source !
tib @ source0 source.buf !
#tib @ source0 source.len !
0 source0 source.id !
0 source0 source.in !
0 source0 source.name !
0 source0 source.name !

\ redefine tib and #tib to use this source record as source-of-truth
source0 source.buf constant tib
source0 source.len constant #tib

\ build a stack of source buffers as well
128 constant |source.buf|
create source-buffers |source.buf| 8 * allot
source-buffers |source.buf| 8 * + constant source-buffer0
variable 'source-buffer
source-buffer0 'source-buffer !

: take-source-buffer ( -- buf )
  'source-buffer @ |source.buf| -
  dup 'source-buffer !
;
: return-source-buffer ( -- )
  |source.buf| 'source-buffer +!
;

: add-file-source ( name name# fid -- )
  @source
  >in @ over source.in !
  |source| -
  dup 'source !
  >r
  take-source-buffer r@ source.buf !
  0 r@ source.len !
  0 r@ source.in !
  r@ source.id !
  r@ source.name# !
  r> source.name !
;

: drop-source ( -- )
  @source
  dup source.id @ close-file throw
  return-source-buffer
  |source| +
  dup source.in @ >in !
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
: source-id @source source.id @ ; host-deferred

\ SOURCE returns the current address and length of the input buffer
: source ( -- c-addr u )
  @source dup source.buf @
  swap source.len @
; host-deferred

\ REFILL tries to pull more data into source,
\ and returns a flag saying whether the source is empty now
: refill ( -- ? )
  0 >in ! \ reset >IN
  @source source.buf @ 128 @source source.id @ ( c-addr u1 fid )
  read-line throw ( u2 more? )
  swap @source source.len ! \ write how much we read
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

\ TODO: implement number parsing in here instead of leaning on the host impl
: binary 2 base ! ;
: decimal 10 base ! ;
: hex 16 base ! ;

: s>number? ( c-addr u -- d ? )
  ?number if 0 -1 else 0 0 0 then
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

 \ string literal
 : s" ( -- c-addr u )
  [char] " parse \ read the quote-delimited string
  compiling?
    if postpone sliteral
    else stemp
    then
; immediate

: bufwrite ( c-addr c -- c-addr )
  over c! 1+
;

\ string literal but with escape chars
: s\" ( -- c-addr u )
  parse-area
  stemp-buffer dup >r \ store head of buffer for l8r
  swap 0 ?do ( src buffer )
    over c@
    dup [char] " = if drop 1 parse-consume leave then
    dup [char] \ =
      if
        drop over 1+ c@
        case
          [char] b of 8 bufwrite 2 endof \ bs
          [char] n of 10 bufwrite 2 endof \ nl
          [char] q of 34 bufwrite 2 endof \ quote
          [char] z of 0 bufwrite 2 endof \ null
          [char] x of \ 2-digit hex literal
            base @ >r 16 base ! \ switch to hex for a sec
            over 2 + 2 ?number =0 throw \ parse a 2-digit hex number
            r> base ! \ back to how we started
            bufwrite 4
          endof
          ( default ) bufwrite 2 0 \ just write the char
        endcase
      else bufwrite 1
      then ( .. src buffer' incr )
    rot over + -rot ( .. src' buffer' incr )
    dup parse-consume
  +loop ( src' buffer' )
  nip r> tuck - ( c-addr u )
  compiling? if postpone sliteral then \ compile into a def if we're compiling
; immediate

\ Now that we have a source, we have a concept of a "current directory"
: current-file ( -- c-addr u )
  @source
  begin dup source0 <>
  while dup source.name# @ =0
  while |source| +
  repeat then
  dup source.name @ swap source.name# @
;

\ "foo/bar/baz" -> "foo/bar/"
: directory-of ( c-addr u -- c-addr u )
  begin dup
  while 2dup + 1- c@ separator-char <>
  while 1-
  repeat then
;

: current-directory ( -- c-addr u )
  current-file directory-of
;

\ Add support for resolving relative paths.

: relative? ( c-addr u -- ? )
  if c@ [char] . =
  else drop false
  then
;

create pathbuf 80 allot
variable pathbuf#
0 pathbuf# !

: push-path-segment ( c-addr u -- )
  pathbuf pathbuf# @ + swap
  dup pathbuf# +!
  move
;
: drop-path-segment ( -- )
  pathbuf pathbuf# @ \ start with the current path so far
  dup =0
    if 814 throw \ can't traverse above the root
    else 1- \ drop the trailing separator
    then
  directory-of
  nip pathbuf# !
;
: push-separator-if-needed ( -- )
  \ if the path is empty, we're at the top level
  \ don't bother adding a separator there
  pathbuf# @ ?dup =0 if exit then
  pathbuf + 1-
  dup c@ separator-char =
    if drop \ if it ends in a separator, do nothing
    else
      separator-char swap 1+ c!
      1 pathbuf# +!
    then
;

: next-segment ( c-addr u -- segment-addr segment-u rest-addr rest-u )
  separator-char split 
;

: resolve-relative-path ( c-addr u -- c-addr u )
  2dup relative? =0 if exit then
  0 pathbuf# !
  current-directory push-path-segment
  begin next-segment dup
  while
    push-separator-if-needed
    2dup s" ." str=
      if 2drop \ do nothing for current dir
      else 2dup s" .." str=
        if 2drop drop-path-segment
        else push-path-segment
        then
      then
  repeat
  2drop 2drop
  pathbuf pathbuf# @
;

\ update the file builtins to respect relative paths
: create-file >r resolve-relative-path r> create-file ;
: open-file >r resolve-relative-path r> open-file ;

