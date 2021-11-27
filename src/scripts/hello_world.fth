include src/scripts/assembler.fth

create program program-size allot
program init-program

\ imports
program wasi-import: proc_exit s-

\ function
1 program program>func vec>size !
s" -" program +type program program>func push-uint \ [] -> []
\ code
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
1 program +start \ function 1

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile close-file
bye
