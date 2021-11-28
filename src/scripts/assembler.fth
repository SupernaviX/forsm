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

struct
  cell field buf.data
  cell field buf.len
  cell field buf.capacity
end-struct |buf|

: buf>contents ( buf -- c-addr u )
  dup buf.data @ swap buf.len @
;
: init-buf ( address capacity -- )
  2dup swap buf.capacity !
  allocate throw over buf.data !
  0 swap buf.len !
;
: free-buf ( buf -- )
  buf.data @ free throw
;
: clear-buf ( buf -- )
  0 swap buf.len !
;
: grow-buf ( buf -- )
  dup buf.capacity @ 2* swap \ get new capacity
  2dup buf.capacity ! \ track it
  dup buf.data @ rot resize throw \ grow the data
  swap buf.data ! \ store the grown data
;
: write-buf ( buf fid -- )
  swap buf>contents rot write-file throw
;

: reserve-space ( u buf -- new-space )
  tuck buf.len @ >r \ track OG length for return
  r@ + \ and new length needed
  begin over buf.capacity @ over < \ grow while we gotta grow
  while over grow-buf
  repeat
  over buf.len !  \ update the length
  buf.data @ r> + \ return the head
;
: push-byte ( c buf -- )
  1 swap reserve-space c!
;
: push-cell ( n buf -- )
  4 swap reserve-space !
;
: push-bytes ( c-addr u buf -- )
  over swap reserve-space
  swap cmove
;

struct
  |buf| field vec.buf 
  cell field vec.size
end-struct |vec|

: init-vec ( address capacity -- )
  over 0 swap vec.size !
  init-buf
;
\ length of the compiled vector in bytes
: vec>length ( vec -- u )
  dup vec.size @ uleb128 nip
  swap buf.len @ +
;
: free-vec ( addr -- ) free-buf ;
: clear-vec ( addr -- )
  0 over vec.size !
  clear-buf
;
: write-vec ( vec fid -- )
  2dup swap vec.size @ uleb128 rot write-file throw
  write-buf
;

struct
  |buf| field ivec.buf
  |buf| field ivec.addresses
end-struct |ivec|

: ivec>size ( addr -- u )
  ivec.addresses buf.len @ 2 rshift 1-
;
: ivec>length ( addr -- u )
  dup ivec>size uleb128 nip
  swap buf.len @ +
;
: ivec-add-entry ( addr -- index )
  dup buf.len @
  over ivec.addresses push-cell
  ivec>size 1-
;
: ivec[] ( index addr -- c-addr u )
  tuck ivec.addresses buf.data @ swap cells + ( addr >start-offset )
  dup @ swap cell + @ over - ( addr start-offset u )
  rot buf.data @ rot + swap
;
: ivec-find ( c-addr u addr -- index | -1 )
  >r
  r@ ivec>size 1- \ find the index of the last entry
  begin dup -1 >
  while ( c-addr u offset )
    >r 2dup r@ -rot r> \ clone the stack
    r@ ivec[] \ get the ith item in the vector
    str= =0 \ keep going unless it isn't equal
  while 1-
  repeat then
  r> drop nip nip
;

: init-ivec ( addr capacity -- )
  2dup init-buf
  over ivec.addresses swap init-buf
  0 swap ivec.addresses push-cell
;
: free-ivec ( addr -- )
  dup ivec.addresses free-buf
  free-buf
;
: clear-ivec ( addr -- )
  dup ivec.addresses clear-buf
  clear-buf
;
: write-ivec ( ivec fid -- )
  2dup swap ivec>size uleb128 rot write-file throw
  write-buf
;

struct
  |ivec| field program.type
  |vec| field program.import
  |vec| field program.memory
  |vec| field program.global
  |vec| field program.func
  |vec| field program.code
  |buf| field program.start
end-struct |program|

variable current-program
: program! current-program ! ;

: init-program ( address -- )
  dup program.type 8 init-ivec
  dup program.import 32 init-vec
  dup program.memory 8 init-vec
  dup program.global 8 init-vec
  dup program.func 8 init-vec
  dup program.code 8 init-vec
  program.start 1 init-buf
;
: free-program ( address -- )
  dup program.type free-ivec
  dup program.import free-vec
  dup program.memory free-vec
  dup program.global free-vec
  dup program.func free-vec
  dup program.code free-vec
  program.start free-buf
;
: write-section ( index addr fid -- )
  over buf.len @ =0
    if 2drop drop exit
    then
  tuck 2swap write-uint
  over buf.len @ over write-uint
  write-buf
;
: write-vec-section ( index addr fid -- )
  over buf.len @ =0
    if 2drop drop exit
    then
  tuck 2swap write-uint
  over vec>length over write-uint
  write-vec
;
: write-ivec-section ( index addr fid -- )
  over buf.len @ =0
    if 2drop drop exit
    then
  tuck 2swap write-uint
  over ivec>length over write-uint
  write-ivec
;
: write-program ( address fid -- )
  >r
  s\" \zasm\x01\z\z\z" r@ write-file throw
  1 over program.type r@ write-ivec-section
  2 over program.import r@ write-vec-section
  3 over program.func r@ write-vec-section
  5 over program.memory r@ write-vec-section
  6 over program.global r@ write-vec-section
  8 over program.start r@ write-section
  10 swap program.code r> write-vec-section
;

create compilebuf |buf| allot
compilebuf 256 init-buf

: compile-start ( -- ) compilebuf clear-buf ;
: compile-stop ( -- c-addr u )
  compilebuf buf.data @ compilebuf buf.len @
;
: uncompile ( u -- ) negate compilebuf buf.len +! ;
: compile-byte ( c -- ) compilebuf push-byte ;
: compile-bytes ( c-addr u -- ) compilebuf push-bytes ;
: compile-uint ( u -- ) uleb128 compile-bytes ;
: compile-sint ( n -- ) sleb128 compile-bytes ;
: compile-string ( c-addr u -- )
  dup compile-uint
  compile-bytes
;
16 base !
: encode-primitive ( c -- c )
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
    over c@ encode-primitive compile-byte
    1 /string
  repeat
  drop
;

: compile-limits ( min max? -- )
  ?dup
    if 1 compile-byte swap compile-uint compile-uint
    else 0 compile-byte compile-uint
    then
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

: +type ( c-addr u -- index )
  current-program @ program.type >r
  parse-signature
  2dup r@ ivec-find dup -1 =
    if \ type not found, add it
      drop
      r@ push-bytes
      r> ivec-add-entry
    else \ type found, return its index
      nip nip
      r> drop
    then
;

: +wasi-import ( c-addr u type -- )
  current-program @ program.import >r
  1 r@ vec.size +!
  compile-start
  s" wasi_snapshot_preview1" compile-string
  -rot compile-string \ compile the import name
  0 compile-uint      \ type is function
  compile-uint        \ encode the function signature
  compile-stop r> push-bytes
;

: wasi-import: ( -- )
  parse-name 
  parse-name +type
  +wasi-import
;

: +memory ( min ?max -- )
  compile-start compile-limits compile-stop
  current-program @ program.memory
  1 over vec.size +!
  push-bytes
;

: +start ( index -- )
  compile-start compile-uint compile-stop
  current-program @ program.start push-bytes
;

16 base !
: end         ( -- )              0b compile-byte ;
: call        ( func -- )         10 compile-byte compile-uint ;
: local.get   ( u -- )            20 compile-byte compile-uint ;
: local.set   ( u -- )            21 compile-byte compile-uint ;
: local.tee   ( u -- )            22 compile-byte compile-uint ;
: global.get  ( u -- )            23 compile-byte compile-uint ;
: global.set  ( u -- )            24 compile-byte compile-uint ;
: i32.load    ( align offset -- ) 28 compile-byte swap compile-uint compile-uint ;
: i64.load    ( align offset -- ) 29 compile-byte swap compile-uint compile-uint ;
: i32.store   ( align offset -- ) 36 compile-byte swap compile-uint compile-uint ;
: i64.store   ( align offset -- ) 37 compile-byte swap compile-uint compile-uint ;
: i32.const   ( n -- )            41 compile-byte compile-sint ;
: i32.add     ( -- )              6a compile-byte ;
: i32.sub     ( -- )              6b compile-byte ;
: i32.mul     ( -- )              6c compile-byte ;
: i32.div_s   ( -- )              6d compile-byte ;
a base !

: global: ( -- )
  compile-start
  parse-name \ next string is like "c" or "cmut"
  over c@ encode-primitive compile-byte
  1 /string s" mut" str= if 1 else 0 then compile-byte
;

: global; ( -- index )
  end compile-stop
  current-program @ program.global
  dup vec.size @ >r
  1 over vec.size +!
  push-bytes
  r>
;

: func: ( -- )
  current-program @ >r
  parse-name +type
  1 r@ program.func vec.size +!
  uleb128 r@ program.func push-bytes
  1 r> program.code vec.size +!
  compile-start
  0 compile-uint \ default to 0 locals
;

create localvec |vec| allot
localvec 16 init-vec
: locals ( -- )
  localvec clear-vec
  parse-name \ looks like "ssdsd"
  begin ?dup
  while
    1 localvec vec.size +!
    over c@ >r
    2dup r@ prefix-length
    dup uleb128 localvec push-bytes /string
    r> encode-primitive localvec push-byte
  repeat drop
  1 uncompile \ remove the "0 locals" we started with
  localvec vec.size @ compile-uint
  localvec buf.data @ localvec buf.len @ compile-bytes
;

: func; ( -- )
  current-program @ program.code >r
  end compile-stop
  dup uleb128 r@ push-bytes
  r> push-bytes
;

: latest-func ( -- u )
  current-program @
  dup program.import vec.size @
  swap program.func vec.size @ + 1-
;
: is-start ( -- )
  latest-func +start
;