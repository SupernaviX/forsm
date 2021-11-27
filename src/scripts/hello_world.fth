include src/scripts/assembler.fth

create program |program| allot
program init-program
program program!

\ imports
wasi-import: proc_exit {c-}

\ functions
func: {cc-c}
  0 local.get
  1 local.get
  i32.add
func;
latest-func constant sum

func: {-}
  23 i32.const
  46 i32.const
  sum call
  0 call \ call function 0, which is proc_exit
func; is-start

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
bye
