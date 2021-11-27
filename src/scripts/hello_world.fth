include src/scripts/assembler.fth

create program program-size allot
program init-program
program program!

\ imports
wasi-import: proc_exit {s-}

\ function
func: {-}
  69 i32.const \ teehee
  0 call \ call function 0, which is proc_exit
func; is-start

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
bye
