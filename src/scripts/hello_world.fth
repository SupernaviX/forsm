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

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !

: compile-byte ( c -- ) outfile @ emit-file throw ;
: compile-bytes ( c-addr u -- ) outfile @ write-file throw ;
: compile-uint ( u -- ) uleb128 compile-bytes ;
: compile-sint ( n -- ) sleb128 compile-bytes ;
: compile-string ( c-addr u ) dup compile-uint compile-bytes ;

16 base !

s\" \zasm\x01\z\z\z" compile-bytes

\ type section
1 compile-byte
8 compile-uint
2 compile-byte \ two types
s\" \x60\x01\x7f\z" compile-bytes \ type 0: [i32] -> []
s\" \x60\z\z" compile-bytes \ type 1: [] -> []

\ import section
2 compile-byte
24 compile-uint
1 compile-byte \ one import
s" wasi_snapshot_preview1" compile-string \ 23
s" proc_exit" compile-string \ 10
0 compile-byte \ function
0 compile-byte \ type 0

\ func section
3 compile-byte
2 compile-uint
1 compile-byte \ one function
1 compile-byte \ type index 1

\ start section
8 compile-byte
1 compile-uint
1 compile-byte \ function 1

\ code section
a compile-byte
9 compile-uint
1 compile-byte \ one function
7 compile-uint \ size of function
0 compile-byte \ no locals
41 compile-byte \ i32.const
45 compile-sint \ teehee
10 compile-byte \ call
0 compile-byte \ function 0 (the import)
b compile-byte \ end

outfile close-file
bye

