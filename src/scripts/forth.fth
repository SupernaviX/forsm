include ./assembler.fth

create program |program| allot
program init-program
program program!

wasi-import: proc_exit {c-}

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
512 constant DICT_SIZE
0 datasec: DICT_START i32.const datasec;
: dictbuf ( -- buf ) literal databuf[] ;
: dict[] ( u -- u ) DICT_START - dictbuf buf[] ;
DICT_SIZE dictbuf init-to-zero

\ Utilities for manually constructing the data dictionary
: v-@ ( u -- n ) dict[] @ ;
: v-! ( n u -- ) dict[] ! ;
: v-+! ( n u -- ) dict[] +! ;
: v-c@ ( u -- c ) dict[] c@ ;
: v-c! ( c u -- ) dict[] c! ;
: v-name>xt ( v-nt -- v-xt )
  dup v-c@ 1+ aligned + cell +
;
: v-name>u ( v-nt -- u ) v-c@ 31 and ;
: v-name>string ( v-nt -- vc-addr u ) dup 1+ swap v-name>u ;
: v-name>backword ( v-nt -- v-nt ) dup v-name>u 1+ + aligned v-@ ;

\ variable and constant support
func: {c-}
  0 local.get (push) call
func; make-callable constant (dovar)
func: {c-}
  0 local.get 0 cell.load (push) call
func; make-callable constant (docon)

\ manually compile a "CP" variable
DICT_START cell + \ leave one cell at the start for the "main" XT
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
  while over c@ v-c, 1 /string
  repeat drop
  >cp v-@ aligned >cp v-!
  >latest v-@ v-,
  r> >latest v-!
;
: v-latestxt ( -- v-xt )
  >latest v-@ v-name>xt
;
: make-native ( func -- )
  parse-name v-header
  make-callable v-,
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

\ the instruction pointer, (docol) and exit give us functions
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
func; make-native exit

: make-colon ( -- )
  parse-name v-header
  (docol) v-,  
;

\ literals are stored inline so they require messing with the IP
func: {c-}
  ip@ 0 local.tee
  4 cell.load (push) call
  0 local.get 8 add ip!
func; make-native lit

\ branches!
func: {c-}
  ip@ 4 cell.load ip!
func; make-native branch

\ conditional branches!
func: {c-}
  (pop) call i32.eqz
    blocktype: c if_ ip@ 4 cell.load
    else_ ip@ 8 add
    end
  ip!
func; make-native ?branch

\ exit with some status code
\ for now, the exit code is the only functioning output
func: {c-}
  (pop) call 0 call
next func; make-native abort

func: {c-}
  stack@ 4 sub 0 local.tee
  0 local.get 4 cell.load 0 cell.store
  0 local.get stack!
next func; make-native dup

func: {c-}
  (pop) call (pop) call
  i32.add
  (push) call
next func; make-native +

func: {c-}
  (pop) call (pop) call
  i32.mul
  (push) call
next func; make-native *

funcref# @ 0 +funcref-table

func: {-} \ inner interpreter
  blocktype: 0 loop_
    ip@ 0 cell.load (execute) call
    0 br
  end
func; is-start

\ Use a virtual IP and return stack to emulate words during compilation
variable v-ip
create v-rstack 256 allot
here constant v-r0
variable v-rp
v-r0 v-rp !
: v->r ( value -- ) -4 v-rp +! v-rp @ !  ;
: v-r> ( -- value ) v-rp @ @ 4 v-rp +! ;
: v-rdepth ( -- u ) v-r0 v-rp @ - 2/ 2/ ;

: callable' ( -- callable )
  ['] lit , v-' v-@ ,
; immediate

\ Given an XT from the virtual interpreter, run it
: v-execute' ( v-xt -- )
  >r r@ v-@
  case
    (docol) of
      v-ip @ v->r \ store current ip on the return stack
      r@ cell + v-ip ! \ new ip is the colon definition
    endof
    callable' exit of
      v-r> cell + v-ip ! \ pop return stack + 4 into ip
    endof
    callable' lit of
      v-ip @ cell + v-@ \ read literal from next cell
      2 cells v-ip +! \ jump past it
    endof
    callable' branch of
      v-ip @ cell + v-@ v-ip !
    endof
    callable' ?branch of =0
      if v-ip @ cell + v-@ v-ip !
      else 2 cells v-ip +!
      then
    endof
    cell v-ip +!  \ everything below this line just increments IP normally
    (dovar) of r@ cell + endof
    (docon) of r@ cell + v-@ endof
    callable' dup of dup endof
    callable' + of + endof
    callable' * of * endof
    ( default )
      ." Callable not supported: " dup . cr
      140 throw
  endcase
  r> drop
;
\ Given an XT from the virtual interpreter, run it
: v-execute ( v-xt -- )
  v-execute' \ execute the first XT
  begin v-rdepth \ if it was a colon definition,
  while v-ip @ v-@ v-execute' \ keep executing until it's done
  repeat
;

\ handwritten colon definitions currently look like this
make-colon square
  v-' dup v-,
  v-' * v-,
v-' exit v-,

make-colon condtest
  v-' ?branch v-, v-here 0 v-,
    v-' lit v-, 5 v-,
  v-' branch v-, v-here 0 v-, swap v-here swap v-!
    v-' lit v-, 6 v-,
  v-here swap v-!
v-' exit v-,

make-colon main
  v-' lit v-, 8 v-,
  v-' square v-,
  v-' lit v-, -1 v-,
  v-' condtest v-,
  v-' + v-,
  v-' abort v-,
v-' exit v-,

v-' main DICT_START v-!

3 v-' square v-execute .
-1 v-' condtest v-execute .
0 v-' condtest v-execute .

variable outfile
s" bin/forth.wasm" w/o create-file throw outfile !
program outfile @ write-program
program free-program
outfile @ close-file
." File generated! " cr
bye
