: uleb128 ( u -- c-addr u )
  pad swap \ scratchpad to work on
  begin ( pad u )
    dup 127 and 
    swap 7 rshift swap 
    over if 128 or then
    rot tuck c!
    1+ swap
    ?dup =0
  until
  pad tuck -
;

: sleb128 ( n -- c-addr u )
  pad swap
  begin ( pad n )
    dup 127 and
    swap 7 arshift swap ( pad n' byte )
    over case \ we are done if the 7th bit of byte matches every bit of n'
      -1 of dup 64 and <>0 endof
      0 of dup 64 and =0 endof
      ( default ) drop false
    endcase ( pad n' byte done? )
    dup >r =0 if 128 or then
    rot tuck c!
    1+ swap
    r>
  until
  drop
  pad tuck -
;

: write-uint ( u fid -- )
  swap uleb128 rot write-file throw
;

3 cells constant buf-size
: buf>data      0 cells + ;
: buf>len       1 cells + ;
: buf>capacity  2 cells + ;
: init-buf ( address capacity -- )
  2dup swap buf>capacity !
  allocate throw over buf>data !
  0 swap buf>len !
;
: free-buf ( buf -- )
  buf>data @ free throw
;
: clear-buf ( buf -- )
  0 swap buf>len !
;
: grow-buf ( buf -- )
  dup buf>capacity @ 2* swap \ get new capacity
  2dup buf>capacity ! \ track it
  dup buf>data @ rot resize throw \ grow the data
  swap buf>data ! \ store the grown data
;
: write-buf ( address fid -- )
  over buf>data @ rot buf>len @ rot write-file throw
;

: push-byte ( c buf -- )
  dup buf>len @ 1+ over buf>capacity @ >
    if dup grow-buf then
  dup buf>data @ over buf>len @ + -rot \ hold onto copy target for l8r
  1 swap buf>len +! \ increment length
  swap c!
;
: push-bytes ( c-addr u buf -- )
  begin
    2dup buf>len @ + over buf>capacity @ >
  while dup grow-buf
  repeat
  dup buf>data @ over buf>len @ + -rot \ hold onto copy target for l8r
  over swap buf>len +! \ increment length
  cmove
;

: push-uint ( u buf -- ) swap uleb128 rot push-bytes ;
: push-string ( c-addr u buf )
  tuck over uleb128 rot push-bytes
  swap push-bytes
;

buf-size 1 cells + constant vec-size
: vec>size    buf-size + ;
: init-vec ( address capacity -- )
  over 0 swap vec>size !
  init-buf
;
: free-vec ( addr -- ) free-buf ;
: clear-vec ( addr -- )
  0 over vec>size !
  clear-buf
;
\ length of the compiled vector in bytes
: vec-length ( addr -- u )
  dup vec>size @ uleb128 nip
  swap buf>len @ +
;
: write-vec ( addr fid -- )
  2dup swap vec>size @ uleb128 rot write-file throw
  write-buf
;

variable current-program
4 vec-size * buf-size + constant program-size
: program>type 0 vec-size * + ;
: program>import 1 vec-size * + ;
: program>func 2 vec-size * + ;
: program>code 3 vec-size * + ;
: program>start 4 vec-size * + ;
: init-program ( address -- )
  dup program>type 8 init-vec
  dup program>import 32 init-vec
  dup program>func 8 init-vec
  dup program>code 8 init-vec
  program>start 1 init-buf
;
: program! current-program ! ;
: free-program ( address -- )
  dup program>type free-vec
  dup program>import free-vec
  dup program>func free-vec
  dup program>code free-vec
  program>start free-buf
;
: write-section ( address index fid -- )
  tuck write-uint
  over buf>len @ over write-uint
  write-buf
;
: write-vec-section ( address index fid -- )
  tuck write-uint
  over vec-length over write-uint
  write-vec
;
: write-program ( address fid -- )
  >r
  s\" \zasm\x01\z\z\z" r@ write-file throw
  dup program>type 1 r@ write-vec-section
  dup program>import 2 r@ write-vec-section
  dup program>func 3 r@ write-vec-section
  dup program>start 8 r@ write-section
  program>code 10 r> write-vec-section
;

create compilebuf buf-size allot
compilebuf 256 init-buf

: compile-start ( -- ) compilebuf clear-buf ;
: compile-stop ( -- c-addr u )
  compilebuf buf>data @ compilebuf buf>len @
;
: uncompile ( u -- ) negate compilebuf buf>len +! ;
: compile-byte ( c -- ) compilebuf push-byte ;
: compile-bytes ( c-addr u -- ) compilebuf push-bytes ;
: compile-uint ( u -- ) uleb128 compile-bytes ;
: compile-sint ( n -- ) sleb128 compile-bytes ;
: compile-string ( c-addr u -- )
  dup compile-uint
  compile-bytes
;
16 base !
: convert-primitive ( c -- c )
  case
    [char] c of 7f endof
    [char] d of 7e endof
    ( default ) 420 throw \ unrecognized char
  endcase
;
a base !
: compile-primitives ( c-addr u -- )
  dup compile-uint
  begin ?dup
  while
    over c@ convert-primitive compile-byte
    1 /string
  repeat
  drop
;

16 base !
\ accepts signatures like "{cc-d}"
: parse-signature ( c-addr u -- c-addr u )
  swap 1+ swap 2 - \ trim the curlies off 
  [char] - split 2swap
  compile-start
  60 compile-byte
  compile-primitives
  compile-primitives
  compile-stop
;
a base !

: +type ( c-addr u program -- index )
  program>type >r
  parse-signature r@ push-bytes
  r@ vec>size @ dup 1+ r> vec>size !
;

: +wasi-import ( c-addr u type program -- )
  program>import >r
  1 r@ vec>size +!
  compile-start
  s" wasi_snapshot_preview1" compile-string
  -rot compile-string \ compile the import name
  0 compile-uint      \ type is function
  compile-uint        \ encode the function signature
  compile-stop r> push-bytes
;

: wasi-import: ( -- )
  parse-name 
  parse-name current-program @ +type
  current-program @ +wasi-import
;

: +start ( index program -- )
  swap
  compile-start compile-uint compile-stop
  rot program>start push-bytes
;

16 base !
: end ( -- )          0b compile-byte ;
: call ( func -- )    10 compile-byte compile-uint ;
: local.get ( u -- )  20 compile-byte compile-uint ;
: local.set ( u -- )  21 compile-byte compile-uint ;
: local.tee ( u -- )  22 compile-byte compile-uint ;
: i32.const ( n -- )  41 compile-byte compile-sint ;
: i32.add   ( -- )    6a compile-byte ;
: i32.sub   ( -- )    6b compile-byte ;
: i32.mul   ( -- )    6c compile-byte ;
: i32.div_s ( -- )    6d compile-byte ;
a base !

: func: ( -- )
  parse-name current-program @ +type
  1 current-program @ program>func vec>size +!
  current-program @ program>func push-uint
  1 current-program @ program>code vec>size +!
  compile-start
  0 compile-uint \ no support for locals yet
;

create localvec vec-size allot
localvec 16 init-vec
: locals ( -- )
  localvec clear-vec
  parse-name \ looks like "ssdsd"
  begin ?dup
  while
    1 localvec vec>size +!
    over c@ >r
    2dup r@ prefix-length
    dup localvec push-uint /string
    r> convert-primitive localvec push-byte
  repeat drop
  1 uncompile \ remove the "0 locals" we started with
  localvec vec>size @ compile-uint
  localvec buf>data @ localvec buf>len @ compile-bytes
;

: func; ( -- )
  end compile-stop
  current-program @ program>code push-string
;

: latest-func ( -- u )
  current-program @
  dup program>import vec>size @
  swap program>func vec>size @ + 1-
;
: is-start ( -- )
  latest-func current-program @ +start
;