include src/scripts/assembler.fth

create program |program| allot
program init-program
program program!

wasi-import: proc_exit {c-}

variable funcref#
0 funcref# !

1 0 +memory
0 elemsec: 0 i32.const elemsec; elemsec!

\ assembly utils
: add ( n -- )  i32.const i32.add ;
: sub ( n -- )  i32.const i32.sub ;
: cell.load  ( offset -- ) 2 swap i32.load ;
: cell.store ( offset -- ) 2 swap i32.store ;

\ given an XT, execute it
type: {c-} constant callable-type
func: {c-}
  0 local.get
  0 local.get 0 cell.load
  callable-type call_indirect
func; constant (execute)

\ Implement the data stack, with push and pop
global: cmut 256 i32.const global; constant stack
: stack@ ( -- ) stack global.get ;
: stack! ( -- ) stack global.set ;
func: {c-} locals c
  stack@ 4 sub 1 local.tee
  0 local.get 0 cell.store \ store the value
  1 local.get stack! \ update the stack pointer
func; constant (push)
func: {-c} locals c
  stack@ 0 local.tee 0 cell.load \ load the value the SP points to
  0 local.get 4 add stack! \ update the stack pointer
func; constant (pop)

\ Implement the return stack with its own push and pop
global: cmut 128 i32.const global; constant rp
: rp@ ( -- ) rp global.get ;
: rp! ( -- ) rp global.set ;
func: {c-} locals c
  rp@ 4 sub 1 local.tee
  0 local.get 0 cell.store
  1 local.get rp!
func; constant (rpush)
func: {-c} locals c
  rp@ 0 local.tee 0 cell.load
  0 local.get 4 add rp!
func; constant (rpop)

\ Utilities for manually constructing the data dictionary
256 constant DICT_START
0 datasec: DICT_START i32.const datasec;
: dictbuf ( -- buf ) literal databuf[] ;
: dict-here ( -- n ) dictbuf buf.len @ DICT_START + ;
: dict, ( n -- ) dictbuf push-cell ;
: make-callable ( func -- index )
  1 funcref# +!
  +elem
;
: make-native ( func -- address )
  dict-here swap make-callable dict,
;

\ Add another data section named "execution"
\ it's just the place in memory where we start execution
512 constant EXECUTION_START
0 datasec: EXECUTION_START i32.const datasec;
: execbuf ( -- buf ) literal databuf[] ;
: exec, ( n -- ) execbuf push-cell ;

\ the instruction pointer, (docol) and 'exit give us functions
global: cmut EXECUTION_START i32.const global; constant ip
: ip@ ( -- ) ip global.get ;
: ip! ( -- ) ip global.set ;
: next ( -- ) ip@ 4 add ip! ;

func: {c-}
  ip@ (rpush) call
  0 local.get 4 add ip!
func; make-callable constant (docol)
func: {c-}
  (rpop) call 4 add ip!
func; make-native constant 'exit

\ literals are stored inline so they require messing with the IP
func: {c-}
  ip@ 0 local.tee
  4 cell.load (push) call
  0 local.get 8 add ip!
func; make-native constant 'lit

\ exit with some status code
\ for now, the exit code is the only functioning output
func: {c-}
  (pop) call 0 call
next func; make-native constant 'abort

func: {c-}
  stack@ 4 sub 0 local.tee
  0 local.get 4 cell.load 0 cell.store
  0 local.get stack!
next func; make-native constant 'dup

func: {c-}
  (pop) call (pop) call
  i32.add
  (push) call
next func; make-native constant '+

func: {c-}
  (pop) call (pop) call
  i32.mul
  (push) call
next func; make-native constant '*

funcref# @ 0 +funcref-table

func: {-} \ inner interpreter
  blocktype: 0 loop_
    ip@ 0 cell.load (execute) call
    0 br
  end
func; is-start

\ handwritten colon definitions currently look like this
dict-here constant 'square
(docol) dict,
'dup dict,
'* dict,
'exit dict,

dict-here constant 'main
(docol) dict,
'lit dict, 8 dict,
'square dict,
'lit dict, 5 dict,
'+ dict,
'exit dict,

'main exec,
'abort exec,

variable outfile
s" bin/forth.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
bye
