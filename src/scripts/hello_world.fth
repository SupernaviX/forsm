include src/scripts/assembler.fth

create program program-size allot
program init-program

\ type section
s\" \x60\x01\x7f\z" program add-type \ type 0: [i32] -> []
s\" \x60\z\z" program add-type \ type 1: [] -> []

\ import section
s" proc_exit" 0 program add-wasi-import

\ func section
1 program program>func vec>size !
1 program program>func push-uint \ type 1

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

\ start section
1 program set-start \ function 1

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !

program outfile @ compile-program
program free-program
outfile close-file
bye
