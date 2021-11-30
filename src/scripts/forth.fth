include src/scripts/assembler.fth

create program |program| allot
program init-program
program program!

wasi-import: proc_exit {c-}

1 0 +memory

global: cmut 256 i32.const global; constant stack
: stack@ ( -- ) stack global.get ;
: stack! ( -- ) stack global.set ;
: add ( n -- )  i32.const i32.add ;
: sub ( n -- )  i32.const i32.sub ;
: cell.load  ( offset -- ) 2 swap i32.load ;
: cell.store ( offset -- ) 2 swap i32.store ;

func: {c-} locals c
  stack@ 4 sub 1 local.tee
  0 local.get 0 cell.store \ store the value
  1 local.get stack! \ update the stack pointer
func; constant 'push

func: {-c} locals c
  stack@ 0 local.tee 0 cell.load \ load the value the SP points to
  0 local.get 4 add stack! \ update the stack pointer
func; constant 'pop

func: {-} locals c
  stack@ 4 sub 0 local.tee
  0 local.get 4 cell.load 0 cell.store
  0 local.get stack!
func; constant 'dup

func: {-}
  'pop call
  'pop call
  i32.add
  'push call
func; constant '+

func: {-}
  'pop call
  'pop call
  i32.mul
  'push call
func; constant '*

func: {-}
  8 i32.const
  'push call
  'dup call
  '* call
  5 i32.const
  'push call
  '+ call
  'pop call
  0 call
func; is-start

variable outfile
s" bin/forth.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
bye
