include ./assembler.fth

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

256 constant DICT_START
256 constant DICT_SIZE
0 datasec: DICT_START i32.const datasec;
: dictbuf ( -- buf ) literal databuf[] ;
: dict[] ( u -- u ) DICT_START - dictbuf buf[] ;
DICT_SIZE dictbuf init-to-zero

\ Utilities for manually constructing the data dictionary
: v-@ ( u -- n ) dict[] @ ;
: v-! ( n u -- ) dict[] ! ;
variable v-cp
DICT_START v-cp !
: v-here ( -- n ) v-cp @ ;
: v-, ( n -- )
  v-here v-!
  cell v-cp +!
;
: make-callable ( func -- index )
  1 funcref# +!
  +elem
;
: make-native ( func -- address )
  v-here swap make-callable v-,
;

\ Execution starts at the head of the dict.
\ Reserve space for a "call main" instruction there later.
0 v-,

\ the instruction pointer, (docol) and 'exit give us functions
global: cmut DICT_START i32.const global; constant ip
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

\ branches!
func: {c-}
  ip@ 4 cell.load ip!
func; make-native constant 'branch

\ conditional branches!
func: {c-}
  (pop) call i32.eqz
    blocktype: c if_ ip@ 4 cell.load
    else_ ip@ 8 add
    end
  ip!
func; make-native constant '?branch

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
v-here constant 'square (docol) v-,
  'dup v-,
  '* v-,
'exit v-,

v-here constant 'condtest (docol) v-,
  '?branch v-, v-here 0 v-,
    'lit v-, 5 v-,
  'branch v-, v-here 0 v-, swap v-here swap v-!
    'lit v-, 6 v-,
  v-here swap v-!
'exit v-,

v-here constant 'main (docol) v-,
  'lit v-, 8 v-,
  'square v-,
  'lit v-, -1 v-,
  'condtest v-,
  '+ v-,
  'abort v-,
'exit v-,

'main DICT_START v-!

variable outfile
s" bin/forth.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
." File generated! " cr
bye
