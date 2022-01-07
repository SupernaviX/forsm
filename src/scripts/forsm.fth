include ./assembler.fth

\ rough memory map
hex
\ the first 16 bytes are unused so that 0 is never a valid pointer
0010 constant TIB_BASE \ input buffer
0100 constant DICT_BASE \ The dictionary. The cell AT this address is main.
ed00 constant PARAM_STACK_BASE \ the HIGHEST address in the param stack (stacks grow down)
f100 constant RETURN_STACK_BASE \ likewise for the return stack
f100 constant HEAP_BASE \ so it's safe for HEAP_BASE to start at the same addr as the return stack
decimal

DICT_BASE TIB_BASE - constant TIB_CAPACITY

create program |program| allot
program init-program
program program!

\ WASI imports
wasi-import: proc_exit {c-} constant (proc-exit)
wasi-import: args_get {cc-c} constant (args-get)
wasi-import: args_sizes_get {cc-c} constant (args-sizes-get)
wasi-import: fd_read {cccc-c} constant (fd-read)
wasi-import: fd_write {cccc-c} constant (fd-write)
wasi-import: path_open {cccccddcc-c} constant (path-open)
wasi-import: fd_close {c-c} constant (fd-close)
wasi-import: fd_prestat_get {cc-c} constant (fd-prestat-get)
wasi-import: fd_prestat_dir_name {ccc-c} constant (fd-prestat-dir-name)
wasi-import: fd_fdstat_get {cc-c} constant (fd-fdstat-get)
variable funcref#
0 funcref# !

1 0 +memory
0 elemsec: 0 i32.const elemsec; elemsec!

: make-callable ( func -- index )
  1 funcref# +!
  +elem
;

\ assembly utils
: add ( n -- )  i32.const i32.add ;
: sub ( n -- )  i32.const i32.sub ;
: cell.load     ( offset -- ) 2 swap i32.load ;
: cell.store    ( offset -- ) 2 swap i32.store ;
: byte.load     ( offset -- ) 0 swap i32.load8_u ;
: byte.store    ( offset -- ) 0 swap i32.store8 ;
: 2cell.load    ( offset -- ) 2 swap i64.load ;
: 2cell.store   ( offset -- ) 2 swap i64.store ;
: double.load   ( offset -- ) 2cell.load 32 i64.const i64.rotl ;
: double.store  ( offset -- ) 32 i64.const i64.rotl 2cell.store ;

\ given an XT, execute it
type: {c-} constant callable-type
func: {c-}
  0 local.get
  0 local.get 0 cell.load
  callable-type call_indirect
func; constant (execute)

\ Implement the data stack, with push and pop
global: cmut PARAM_STACK_BASE i32.const global; constant stack
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
global: cmut RETURN_STACK_BASE i32.const global; constant rp
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

\ Implement the instruction pointer
global: cmut DICT_BASE i32.const global; constant ip
: ip@ ( -- ) ip global.get ;
: ip! ( -- ) ip global.set ;
: next ( -- ) ip@ 4 add ip! ;

: grow-if-needed ( target buf -- )
  tuck buf.len @ - cell + dup >0
    if 0 swap rot push-byte-repeating
    else 2drop
    then
;

\ in-memory data section for the emulated TIB
create v-tib TIB_CAPACITY allot
: tib[] ( u -- u )
  TIB_BASE - v-tib +
;

\ data section for the dictionary
0 datasec: DICT_BASE i32.const datasec;
: dictbuf ( -- buf ) literal databuf[] ;
: dict[] ( u -- u )
  DICT_BASE -
  dup dictbuf grow-if-needed
  dictbuf buf[]
;

\ data section for the heap
0 datasec: HEAP_BASE i32.const datasec;
: heapbuf ( -- buf ) literal databuf[] ;
: heap[] ( u -- u )
  HEAP_BASE -
  dup heapbuf grow-if-needed
  heapbuf buf[]
;

: vaddr>addr ( u -- u )
  dup HEAP_BASE >= if
    heap[] exit
  then
  dup DICT_BASE >= if
    dict[] exit
  then
  dup TIB_BASE >= if
    tib[] exit
  then
  ." Cannae write to wee memory address " . cr
  -20 throw
;

\ Utilities for manually constructing the data dictionary
: v-@ ( u -- n ) vaddr>addr @ ;
: v-! ( n u -- ) vaddr>addr ! ;
: v-+! ( n u -- ) vaddr>addr +! ;
: v-c@ ( u -- c ) vaddr>addr c@ ;
: v-c! ( c u -- ) vaddr>addr c! ;
: v-cset ( c u -- ) vaddr>addr cset ;
: v-creset ( c u -- ) vaddr>addr creset ;
: v-name>u ( v-nt -- u ) v-c@ 31 and ;
: v-name>xt ( v-nt -- v-xt )
  dup v-name>u 1+ aligned + cell +
;
: v-name>string ( v-nt -- vc-addr u ) dup 1+ swap v-name>u ;
: v-name>backword ( v-nt -- v-nt ) dup v-name>u 1+ + aligned v-@ ;
: v-name>immediate? ( v-nt -- ? ) v-c@ 64 and <>0 ;

\ variable and constant support
func: {c-}
  0 local.get 4 add (push) call
next func; make-callable constant (dovar)
func: {c-}
  0 local.get 4 cell.load (push) call
next func; make-callable constant (docon)

\ manually compile a "CP" variable
DICT_BASE cell + \ leave one cell at the start for the "main" XT
dup \ hold onto this address for later
2 over v-c! 1+
char C over v-c! 1+
char P over v-c! 1+
aligned
0 over v-! cell +
(dovar) over v-! cell +
dup cell + swap v-!

\ use that variable for some compilation utilities
dup v-name>xt cell + constant >cp

: v-here ( -- n ) >cp v-@ ;
: v-, ( n -- )
  v-here v-!
  cell >cp v-+!
;
: v-c, ( n -- )
  v-here v-c!
  1 >cp v-+!
;
: v-align ( -- ) v-here aligned >cp v-! ;

\ while CP's address is on the stack, compile "LATEST" as well
v-here swap ( nt-of-latest nt-of-cp )
6 v-c,
char L v-c,
char A v-c,
char T v-c,
char E v-c,
char S v-c,
char T v-c,
v-align
v-, \ the NT for "CP" is still on top of the stack
(dovar) v-,
dup v-, \ this word's NT is the right value for LATEST
v-name>xt cell + constant >latest

\ with CP and LATEST, we can define a HEADER utility
: v-header ( c-addr u -- )
  v-here >r
  dup v-c,
  begin ?dup
  while over c@ upchar v-c, 1 /string
  repeat drop
  >cp v-@ aligned >cp v-!
  >latest v-@ v-,
  r> >latest v-!
;

: make-native ( func -- )
  parse-name v-header
  make-callable v-,
;
: make-variable ( initial -- )
  parse-name v-header
  (dovar) v-,
  v-,
;
: make-constant ( value -- )
  parse-name v-header
  (docon) v-,
  v-,
;
: v-name= ( c-addr u v-c-addr u -- ? )
  rot over <> if
    drop 2drop false exit
  then
  0 ?do ( c-addr v-c-addr )
    over c@ upchar over v-c@ upchar <> if
      2drop false unloop exit
    then
    1+ swap 1+ swap
  loop
  2drop true
;
: v-find-name ( c-addr u -- v-nt | 0 )
  >latest v-@
  begin ?dup
  while ( c-addr u v-nt )
    >r 2dup r@ v-name>string v-name= if
      2drop r> exit
    then
    r> v-name>backword
  repeat
  2drop false
;
: v-' ( -- xt )
  parse-name 2dup v-find-name ?dup
    if nip nip v-name>xt
    else
      ." Word not available in target forth: " type cr
      140 throw
    then
;
: [v-'] ( -- )
  v-'
  lit lit , ,
; immediate

\ Get the address used to back a variable
: v-body ( -- address )
  v-' cell +
;
: [v-body]
  v-body postpone literal
; immediate

\ (docol) and exit give us functions
func: {c-}
  ip@ (rpush) call
  0 local.get 4 add ip!
func; make-callable constant (docol)
func: {c-}
  (rpop) call 4 add ip!
func; make-native exit

: make-colon ( -- )
  parse-name v-header
  (docol) v-,  
;
: v-immediate ( -- )
  64 >latest v-@ v-cset
;

\ define most of the native words here
include ./forsm/00_native.fth

funcref# @ 0 +funcref-table

func: {-} \ inner interpreter
  blocktype: 0 loop_
    ip@ 0 cell.load (execute) call
    0 br
  end
func; export: func _start

0 export: table __indirect_function_table
0 export: memory memory

include ./forsm/01_definitions.fth
include ./forsm/02_emulator.fth

s" ../prelude/01_core.fth" v-bootstrap
s" ../prelude/02_memory.fth" v-bootstrap
s" ../prelude/03_strings.fth" v-bootstrap
s" ../prelude/04_system.fth" v-bootstrap
s" ../prelude/05_parser.fth" v-bootstrap

s" ./bootstrap-forth.fth" v-bootstrap
v-' main DICT_BASE v-!

variable outfile
s" bin/forth.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
." File generated! " cr
bye
