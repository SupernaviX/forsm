: uleb128 ( n -- c-addr u )
  pad swap \ scratchpad to work on
  begin ( pad n )
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
    swap 7 rshift swap ( pad n' byte )
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
: grow-buf ( buf -- )
  dup buf>capacity @ 2* swap \ get new capacity
  2dup buf>capacity ! \ track it
  dup buf>data @ rot resize throw \ grow the data
  swap buf>data ! \ store the grown data
;
: compile-buf ( address fid -- )
  2dup swap buf>len @ uleb128 rot write-file throw
  over buf>data @ rot buf>len @ rot write-file throw
;

: push-byte ( c buf -- )
  dup buf>len @ 1+ over buf>capacity @ >
    if dup grow-buf then
  dup buf>data @ over buf>len @ + -rot \ hold onto copy target for l8r
  1 swap buf>len +! \ increment length
  swap c!
\  tuck dup buf>data @ swap buf>len @ + c! 
\  1 swap buf>len +!
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
: push-sint ( n buf -- ) swap sleb128 rot push-bytes ;
: push-string ( c-addr u buf )
  tuck over uleb128 rot push-bytes
  swap push-bytes
;


5 buf-size * constant program-size
: program>type 0 buf-size * + ;
: program>import 1 buf-size * + ;
: program>func 2 buf-size * + ;
: program>start 3 buf-size * + ;
: program>code 4 buf-size * + ;
: init-program ( address -- )
  dup program>type 8 init-buf
  dup program>import 32 init-buf
  dup program>func 8 init-buf
  dup program>start 1 init-buf
  program>code 32 init-buf
;
: free-program ( address -- )
  dup program>type free-buf
  dup program>import free-buf
  dup program>func free-buf
  dup program>start free-buf
  program>code free-buf
;
: compile-section ( address index fid -- )
  tuck
  swap uleb128 rot write-file throw
  compile-buf
;
: compile-program ( address fid -- )
  >r
  s\" \zasm\x01\z\z\z" r@ write-file throw
  dup program>type 1 r@ compile-section
  dup program>import 2 r@ compile-section
  dup program>func 3 r@ compile-section
  dup program>start 8 r@ compile-section
  program>code 10 r> compile-section
;

create program program-size allot
program init-program

\ type section
2 program program>type push-uint
s\" \x60\x01\x7f\z" program program>type push-bytes \ type 0: [i32] -> []
s\" \x60\z\z" program program>type push-bytes \ type 1: [] -> []

\ import section
1 program program>import push-uint \ one import
s" wasi_snapshot_preview1" program program>import push-string
s" proc_exit" program program>import push-string
0 program program>import push-uint \ function
0 program program>import push-uint \ type 0

\ func section
1 program program>func push-uint \ one function
1 program program>func push-uint \ type 1

\ start section
1 program program>start push-uint \ function 1

\ code section
16 base !
1 program program>code push-uint \ one function
7 program program>code push-uint \ size of function
0 program program>code push-uint \ no locals
41 program program>code push-byte \ i32.const
45 program program>code push-sint \ teehee
10 program program>code push-byte \ call
0 program program>code push-uint \ function 0 (the import)
0b program program>code push-byte \ end
a base !

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !

program outfile @ compile-program
program free-program
outfile close-file
bye
