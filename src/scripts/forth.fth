include src/scripts/assembler.fth

create program |program| allot
program init-program
program program!

wasi-import: proc_exit {c-}

1 0 +memory
3 0 +funcref-table \ TODO: this needs to be as big as all the elem sections
0 elemsec: 0 i32.const elemsec; elemsec!


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
func; +elem constant 'dup

func: {-}
  'pop call
  'pop call
  i32.add
  'push call
func; +elem constant '+

func: {-}
  'pop call
  'pop call
  i32.mul
  'push call
func; +elem constant '*

: execute-callable [ type: {-} ] literal call_indirect ;
func: {-}
  8 i32.const
  'push call
  'dup i32.const
  execute-callable
  '* i32.const
  execute-callable
  5 i32.const
  'push call
  '+ i32.const
  execute-callable
  'pop call
  0 call
func; is-start

variable outfile
s" bin/forth.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
bye
