include src/scripts/assembler.fth

create program |program| allot
program init-program
program program!

wasi-import: proc_exit {c-}

1 0 +memory
5 0 +funcref-table \ TODO: this needs to be as big as all the elem sections
0 elemsec: 0 i32.const elemsec; elemsec!

0 datasec: 256 i32.const datasec;
: stackdata literal databuf[] ;
: v, stackdata push-cell ;

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

global: cmut 256 i32.const global; constant ip
: ip@ ( -- ) ip global.get ;
: ip! ( -- ) ip global.set ;
: next ( -- ) ip@ 4 add ip! ;

func: {-} locals c
  ip@ 0 local.tee
  4 cell.load 'push call
  0 local.get 8 add ip!
func; +elem constant 'lit

func: {-} locals c
  'pop call 0 call \ exit with some status code
next func; +elem constant 'abort

func: {-} locals c
  stack@ 4 sub 0 local.tee
  0 local.get 4 cell.load 0 cell.store
  0 local.get stack!
next func; +elem constant 'dup

func: {-}
  'pop call 'pop call
  i32.add
  'push call
next func; +elem constant '+

func: {-}
  'pop call 'pop call
  i32.mul
  'push call
next func; +elem constant '*

type: [-} constant callable-type

func: {-} \ inner interpreter
  blocktype: 0 loop_
    ip@ 0 cell.load callable-type call_indirect
    0 br
  end
func; is-start

'lit v, 8 v,
'dup v,
'* v,
'lit v, 5 v,
'+ v,
'abort v,

variable outfile
s" bin/forth.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
bye
