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
      ( default ) false swap
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
: push-byte-repeating ( c u buf -- )
  over swap reserve-space
  swap rot fill
;
: init-to-zero ( size buf -- )
  tuck buf.len @ - over reserve-space drop
  dup buf.data @ swap buf.len @ 0 fill
;
: buf[] ( u buf -- u )
  buf.data @ +
;

struct
  |buf| field vec.buf
  |buf| field vec.addresses
end-struct |vec|

: init-vec ( addr capacity -- )
  2dup init-buf
  over vec.addresses swap init-buf
  0 swap vec.addresses push-cell
;
: free-vec ( addr -- )
  dup vec.addresses free-buf
  free-buf
;

: vec>size ( addr -- u )
  vec.addresses buf.len @ 2 rshift 1-
;
: vec>length ( addr -- u )
  dup vec>size uleb128 nip
  swap buf.len @ +
;
: vec-add-entry ( addr -- index )
  dup buf.len @
  over vec.addresses push-cell
  vec>size 1-
;
: vec[] ( index addr -- c-addr u )
  tuck vec.addresses buf.data @ swap cells + ( addr >start-offset )
  dup @ swap cell + @ over - ( addr start-offset u )
  rot buf.data @ rot + swap
;
: vec-find ( c-addr u addr -- index | -1 )
  >r
  r@ vec>size 1- \ find the index of the last entry
  begin dup -1 >
  while ( c-addr u offset )
    >r 2dup r@ -rot r> \ clone the stack
    r@ vec[] \ get the ith item in the vector
    str= =0 \ keep going unless it isn't equal
  while 1-
  repeat then
  r> drop nip nip
;

: write-vec ( vec fid -- )
  2dup swap vec>size uleb128 rot write-file throw
  write-buf
;

struct
  |vec| field elemsec.elems
  |buf| field elemsec.prefix
end-struct |elemsec|

: init-elemsec ( prefix-addr prefix-u addr capacity -- )
  over elemsec.elems swap init-vec
  elemsec.prefix 2dup swap init-buf push-bytes
;

struct
  |buf| field datasec.data
  |buf| field datasec.prefix
end-struct |datasec|

: init-datasec ( prefix-addr prefix-u addr capacity )
  over datasec.data swap init-buf
  datasec.prefix 2dup swap init-buf push-bytes
;

struct
  |vec| field program.type
  |vec| field program.import
  |vec| field program.func
  |vec| field program.table
  |vec| field program.memory
  |vec| field program.global
  |vec| field program.export
  |vec| field program.elem
  |vec| field program.code
  |vec| field program.data
  cell field program.start
end-struct |program|

variable current-program
: program! current-program ! ;
variable current-elemsec
: elemsec! current-elemsec ! ;

: init-program ( address -- )
  dup program.type 8 init-vec
  dup program.import 32 init-vec
  dup program.func 8 init-vec
  dup program.table 8 init-vec
  dup program.memory 8 init-vec
  dup program.global 8 init-vec
  dup program.export 8 init-vec
  dup program.elem 8 init-vec
  -1 over program.start !
  dup program.code 8 init-vec
  program.data 8 init-vec
;
: free-program ( address -- )
  dup program.type free-vec
  dup program.import free-vec
  dup program.func free-vec
  dup program.table free-vec
  dup program.memory free-vec
  dup program.global free-vec
  dup program.export free-vec
  dup program.elem free-vec
  dup program.code free-vec
  program.data free-vec
;

: write-vec-section ( index addr fid -- )
  over buf.len @ =0
    if 2drop drop exit
    then
  tuck 2swap write-uint
  over vec>length over write-uint
  write-vec
;
: write-start-section ( index addr fid -- )
  over -1 =
    if 2drop drop exit
    then
  tuck 2swap write-uint
  over uleb128 nip over write-uint
  write-uint
;
: elem-section-size ( addr -- u )
\ start with the length of the vector's uleb128-encoded size
  dup vec>size uleb128 nip swap
  buf>contents 0 ?do ( u buf )
    dup elemsec.prefix buf.len @ \ add the length of each elemsec's prefix
    over elemsec.elems vec>length + \ and the length of the elemsec itself
    rot + swap |elemsec| +
  |elemsec| +loop
  drop
;
: write-elem-section ( index addr fid -- )
  over buf.len @ =0
    if 2drop drop exit
    then
  tuck 2swap write-uint
  over elem-section-size over write-uint
  over vec>size over write-uint
  swap buf>contents 0 ?do ( fid buf )
    2dup elemsec.prefix swap write-buf \ write the tableindex + offset
    2dup elemsec.elems swap write-vec \ write the contents
    |elemsec| +
  |elemsec| +loop
  2drop
;
: data-section-size ( addr -- u )
  dup vec>size uleb128 nip swap
  buf>contents 0 ?do ( u buf )
    dup datasec.prefix buf.len @ \ add the length of each datasec's prefix
    over datasec.data buf.len @ uleb128 nip + \ and hte length of the data's size
    over datasec.data buf.len @ + \ and the length of the data itself
    rot + swap |datasec| +
  |datasec| +loop
  drop
;
: write-data-section ( index addr fid -- )
  over buf.len @ =0
    if 2drop drop exit
    then
  tuck 2swap write-uint
  over data-section-size over write-uint
  over vec>size over write-uint
  swap buf>contents 0 ?do ( fid buf )
    2dup datasec.prefix swap write-buf
    2dup datasec.data buf.len @ swap write-uint
    2dup datasec.data swap write-buf
    |datasec| +
  |datasec| +loop
  2drop
;
: write-program ( address fid -- )
  >r
  s\" \zasm\x01\z\z\z" r@ write-file throw
  1 over program.type r@ write-vec-section
  2 over program.import r@ write-vec-section
  3 over program.func r@ write-vec-section
  4 over program.table r@ write-vec-section
  5 over program.memory r@ write-vec-section
  6 over program.global r@ write-vec-section
  7 over program.export r@ write-vec-section
  8 over program.start @ r@ write-start-section
  9 over program.elem r@ write-elem-section
  10 over program.code r@ write-vec-section
  11 swap program.data r> write-data-section
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
hex
: encode-primitive ( c -- c )
  case
    [char] c of 7f endof
    [char] d of 7e endof
    ( default ) 420 throw \ unrecognized char
  endcase
;
decimal
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

hex
\ accepts signatures like "{cc-d}"
: parse-signature ( c-addr u -- c-addr u )
  swap 1+ swap 2 - \ trim the curlies off 
  [char] - split
  compile-start
  60 compile-byte
  compile-primitives
  compile-primitives
  compile-stop
;
decimal

: +type ( c-addr u -- index )
  current-program @ program.type >r
  parse-signature
  2dup r@ vec-find dup -1 =
    if \ type not found, add it
      drop
      r@ push-bytes
      r> vec-add-entry
    else \ type found, return its index
      nip nip
      r> drop
    then
;

: type: ( -- index )
  parse-name +type
;

: blocktype: ( -- index )
  char
  dup [char] 0 =
    if [ base @ 16 base ! ] drop 40 [ base ! ]
    else encode-primitive
    then
;

: +wasi-import ( c-addr u type -- index )
  current-program @ program.import >r
  compile-start
    s" wasi_snapshot_preview1" compile-string
    -rot compile-string \ compile the import name
    0 compile-uint      \ type is function
    compile-uint        \ encode the function signature
  compile-stop
  r@ push-bytes
  r> vec-add-entry
;

: wasi-import: ( -- index )
  parse-name 
  parse-name +type
  +wasi-import
;

: +funcref-table ( min ?max -- )
  current-program @ program.table >r
  compile-start
    [ base @ 16 base ! ] 70 [ base ! ] compile-byte
    compile-limits
  compile-stop
  r@ push-bytes
  r> vec-add-entry drop
;

: +memory ( min ?max -- )
  current-program @ program.memory >r
  compile-start compile-limits compile-stop
  r@ push-bytes
  r> vec-add-entry drop
;

: +export ( c-addr u index type -- )
  current-program @ program.export >r
  compile-start
    2swap compile-string  \ compile the export name
    compile-uint          \ encode the type
    compile-uint          \ encode the index
  compile-stop
  r@ push-bytes
  r> vec-add-entry drop
;

: export: ( index -- )
  parse-name
  2dup s" func" str= if 0 then
  2dup s" table" str= if 1 then
  2dup s" memory" str= if 2 then
  2dup s" global" str= if 3 then
  -rot 2drop
  parse-name 2swap +export
;

: +elem ( func -- index )
  current-program @ program.elem buf.data @ current-elemsec @ |elemsec| * + >r
  compile-start compile-uint compile-stop
  r@ push-bytes
  r> vec-add-entry
;

: is-start ( index -- )
  current-program @ program.start !
;

hex
: loop_             ( blocktype -- )    03 compile-byte compile-byte ;
: if_               ( blocktype -- )    04 compile-byte compile-byte ;
: else_             ( -- )              05 compile-byte ;
: end               ( -- )              0b compile-byte ;
: br                ( label -- )        0c compile-byte compile-uint ;
: call              ( func -- )         10 compile-byte compile-uint ;
: call_indirect     ( type -- )         11 compile-byte compile-uint 0 compile-byte ;
: select            ( -- )              1b compile-byte ;
: local.get         ( u -- )            20 compile-byte compile-uint ;
: local.set         ( u -- )            21 compile-byte compile-uint ;
: local.tee         ( u -- )            22 compile-byte compile-uint ;
: global.get        ( u -- )            23 compile-byte compile-uint ;
: global.set        ( u -- )            24 compile-byte compile-uint ;
: i32.load          ( align offset -- ) 28 compile-byte swap compile-uint compile-uint ;
: i64.load          ( align offset -- ) 29 compile-byte swap compile-uint compile-uint ;
: i32.load8_u       ( align offset -- ) 2d compile-byte swap compile-uint compile-uint ;
: i32.store         ( align offset -- ) 36 compile-byte swap compile-uint compile-uint ;
: i64.store         ( align offset -- ) 37 compile-byte swap compile-uint compile-uint ;
: i32.store8        ( align offset -- ) 3a compile-byte swap compile-uint compile-uint ;
: memory.size       ( -- )              3f compile-byte 0 compile-byte ;
: memory.grow       ( -- )              40 compile-byte 0 compile-byte ;
: i32.const         ( n -- )            41 compile-byte compile-sint ;
: i64.const         ( n -- )            42 compile-byte compile-sint ;
: i32.eqz           ( -- )              45 compile-byte ;
: i32.eq            ( -- )              46 compile-byte ;
: i32.ne            ( -- )              47 compile-byte ;
: i32.lt_s          ( -- )              48 compile-byte ;
: i32.lt_u          ( -- )              49 compile-byte ;
: i32.gt_s          ( -- )              4a compile-byte ;
: i32.gt_u          ( -- )              4b compile-byte ;
: i32.le_s          ( -- )              4c compile-byte ;
: i32.le_u          ( -- )              4d compile-byte ;
: i32.ge_s          ( -- )              4e compile-byte ;
: i32.ge_u          ( -- )              4f compile-byte ;
: i32.add           ( -- )              6a compile-byte ;
: i32.sub           ( -- )              6b compile-byte ;
: i32.mul           ( -- )              6c compile-byte ;
: i32.div_s         ( -- )              6d compile-byte ;
: i32.div_u         ( -- )              6e compile-byte ;
: i32.rem_s         ( -- )              6f compile-byte ;
: i32.rem_u         ( -- )              70 compile-byte ;
: i32.and           ( -- )              71 compile-byte ;
: i32.or            ( -- )              72 compile-byte ;
: i32.xor           ( -- )              73 compile-byte ;
: i32.shl           ( -- )              74 compile-byte ;
: i32.shr_s         ( -- )              75 compile-byte ;
: i32.shr_u         ( -- )              76 compile-byte ;
: i64.add           ( -- )              7c compile-byte ;
: i64.sub           ( -- )              7d compile-byte ;
: i64.mul           ( -- )              7e compile-byte ;
: i64.div_s         ( -- )              7f compile-byte ;
: i64.div_u         ( -- )              80 compile-byte ;
: i64.rem_s         ( -- )              81 compile-byte ;
: i64.rem_u         ( -- )              82 compile-byte ;
: i64.and           ( -- )              83 compile-byte ;
: i64.or            ( -- )              84 compile-byte ;
: i64.xor           ( -- )              85 compile-byte ;
: i64.shl           ( -- )              86 compile-byte ;
: i64.shr_s         ( -- )              87 compile-byte ;
: i64.shr_u         ( -- )              88 compile-byte ;
: i64.rotl          ( -- )              89 compile-byte ;
: i32.wrap_i64      ( -- )              a7 compile-byte ;
: i64.extend_i32_s  ( -- )              ac compile-byte ;
: i64.extend_i32_u  ( -- )              ad compile-byte ;
decimal

: elemsec: ( table -- )
  compile-start
  compile-uint
;
: elemsec; ( -- index )
  current-program @ program.elem >r
  end compile-stop
  |elemsec| r@ reserve-space 8 init-elemsec
  r> vec-add-entry
;

: datasec: ( memory -- )
  compile-start
  compile-uint
;
: datasec; ( -- index )
  current-program @ program.data >r
  end compile-stop
  |datasec| r@ reserve-space 8 init-datasec
  r> vec-add-entry
;
: databuf[] ( index -- )
  current-program @ program.data buf.data @ \ get to the buffer of data buffers
  swap |datasec| * + datasec.data \ and return the right one
;

: global: ( -- )
  compile-start
  parse-name \ next string is like "c" or "cmut"
  over c@ encode-primitive compile-byte
  1 /string s" mut" str= if 1 else 0 then compile-byte
;
: global; ( -- index )
  current-program @ program.global >r
  end compile-stop
  r@ push-bytes
  r> vec-add-entry
;

: func: ( -- )
  current-program @ >r
  parse-name +type
  uleb128 r@ program.func push-bytes
  r> program.func vec-add-entry drop
  compile-start
  0 compile-uint \ default to 0 locals
;

create localbuf |buf| allot
variable localbuf-size
localbuf 16 init-buf
: locals ( -- )
  localbuf clear-buf
  0 localbuf-size !
  parse-name \ looks like "ssdsd"
  begin ?dup
  while
    1 localbuf-size +!
    over c@ >r
    2dup r@ prefix-length
    dup uleb128 localbuf push-bytes /string
    r> encode-primitive localbuf push-byte
  repeat drop
  1 uncompile \ remove the "0 locals" we started with
  localbuf-size @ compile-uint
  localbuf buf>contents compile-bytes
;

: func; ( -- index )
  current-program @ >r
  end compile-stop
  dup uleb128 r@ program.code push-bytes
  r@ program.code push-bytes
  r@ program.code vec-add-entry
  r> program.import vec>size +
;
