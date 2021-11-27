include src/scripts/assembler.fth

create program program-size allot
program init-program

\ imports
program wasi-import: proc_exit s-

\ function
1 program program>func vec>size +!
s" -" program +type program program>func push-uint \ [] -> []
\ code
1 program program>code vec>size +!
16 base !
compile-start
0 compile-uint \ no locals
41 compile-byte \ i32.const
45 compile-sint \ teehee
10 compile-byte \ call
0 compile-uint \ function 0 (the import)
0b compile-byte \ end
compile-stop program program>code push-string
a base !

\ start section
1 program +start \ function 1

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile close-file
bye
