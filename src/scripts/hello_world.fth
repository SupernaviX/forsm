include src/scripts/assembler.fth

create program |program| allot
program init-program
program program!

\ imports
wasi-import: proc_exit {c-}

\ memory
1 2 +memory

\ globals
global: cmut
  420 i32.const
global; constant stack

\ functions
func: {cc-}
  locals c
  0 local.get
  1 local.get
  i32.add
  2 local.set
  16 i32.const
  2 local.get
  stack global.set
  2 local.get
  2 0 i32.store
func; constant sum

func: {-}
  23 i32.const
  46 i32.const
  sum call
  16 i32.const
  2 0 i32.load
  0 call \ call function 0, which is proc_exit
func; is-start

variable outfile
s" bin/hello.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
bye
