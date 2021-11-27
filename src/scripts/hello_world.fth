include src/scripts/assembler.fth

create program program-size allot
program init-program

\ type section
2 program program>type vec>size !
s\" \x60\x01\x7f\z" program program>type push-bytes \ type 0: [i32] -> []
s\" \x60\z\z" program program>type push-bytes \ type 1: [] -> []

\ import section
1 program program>import vec>size !
s" wasi_snapshot_preview1" program program>import push-string
s" proc_exit" program program>import push-string
0 program program>import push-uint \ function
0 program program>import push-uint \ type 0

\ func section
1 program program>func vec>size !
1 program program>func push-uint \ type 1

\ start section
1 program program>start push-uint \ function 1

\ code section
16 base !
1 program program>code vec>size !
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
