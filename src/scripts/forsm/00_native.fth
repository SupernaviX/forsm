create parambuf 16 allot
: param[] ( i -- c ) parambuf + c@ ;
variable param#
: param-size ( c -- u )
  [char] d =
    if 2 cells
    else cell
    then
;
: params-size ( -- u )
  0
  param# @ 0 ?do
    i param[] param-size +
  loop
;

\ looks like "ccddc"
: parse-ffi-signature ( c-addr u -- )
  dup param# !
  parambuf swap move
;

: ffi-start ( -- ffi-sys )
  parse-name parse-ffi-signature
  params-size dup ( size size )
  stack@ 0 local.tee \ compiled: store the stack pointer in a local register
  param# @ 0 do
    i param[] [char] d = if
      2 cells - \ displace by the size of a double
      0 local.get dup double.load \ compiled: fetch a double
    else
      cell - \ displace by the size of a cell
      0 local.get dup cell.load \ compiled: fetch a cell
    then
  loop
  drop
;

: ffi-done ( ffi-sys -- )
  cell - \ displace by the size of a cell
  dup cell.store \ compiled: store the result
  0 local.get add stack! \ compiled: move the stack pointer
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

func: {c-}
  (pop) call (execute) call
next func; make-native execute

\ exit with some status code
\ for now, the exit code is the only functioning output
func: {c-}
  (pop) call (proc-exit) call
next func; make-native abort

func: {c-}
  ffi-start cc
  (args-get) call
  ffi-done
next func; make-native args-get

func: {c-}
  ffi-start cc
  (args-sizes-get) call
  ffi-done
next func; make-native args-sizes-get

func: {c-}
  ffi-start cccc
  (fd-read) call
  ffi-done
next func; make-native fd-read

func: {c-}
  ffi-start cccc
  (fd-write) call
  ffi-done
next func; make-native fd-write

func: {c-}
  ffi-start cccccddcc
  (path-open) call
  ffi-done
next func; make-native path-open

func: {c-}
  ffi-start c
  (fd-close) call
  ffi-done
next func; make-native fd-close

func: {c-}
  ffi-start cc
  (fd-prestat-get) call
  ffi-done
next func; make-native fd-prestat-get

func: {c-}
  ffi-start ccc
  (fd-prestat-dir-name) call
  ffi-done
next func; make-native fd-prestat-dir-name

func: {c-}
  ffi-start cc
  (fd-fdstat-get) call
  ffi-done
next func; make-native fd-fdstat-get

func: {c-}
  stack@ 0 local.tee
  0 local.get
  0 cell.load
  0 cell.load
  0 cell.store
next func; make-native @

func: {c-}
  stack@ 0 local.tee
  0 local.get
  0 cell.load
  0 byte.load
  0 cell.store
next func; make-native c@

func: {c-}
  stack@ 0 local.tee
  0 cell.load \ address
  0 local.get 4 cell.load \ value
  0 cell.store
  0 local.get 8 add stack!
next func; make-native !

func: {c-}
  stack@ 0 local.tee
  0 cell.load \ address
  0 local.get 4 cell.load \ value
  0 byte.store
  0 local.get 8 add stack!
next func; make-native c!

func: {c-} locals c
  stack@ 0 local.tee
  0 cell.load 1 local.tee \ address
  0 local.get 4 cell.load \ value
  1 local.get 0 cell.load i32.add \ newvalue
  0 cell.store
  0 local.get 8 add stack!
next func; make-native +!

func: {c-}
  stack@ 0 local.tee
  0 local.get 0 cell.load
  2 i32.const i32.shl
  0 cell.store
next func; make-native cells

func: {c-}
  memory.size (push) call
next func; make-native memory.size
func: {c-}
  (pop) call memory.grow (push) call
next func; make-native memory.grow
HEAP_BASE make-constant heap-base

func: {c-}
  stack@ 4 sub 0 local.tee
  0 local.get 4 cell.load 0 cell.store
  0 local.get stack!
next func; make-native dup

func: {c-} locals c
  stack@ 4 sub 0 local.tee
  4 cell.load 1 local.tee
  blocktype: 0 if_
    0 local.get 1 local.get 0 cell.store
    0 local.get stack!
  end
next func; make-native ?dup

func: {c-}
  stack@ 8 sub 0 local.tee
  0 local.get 8 2cell.load 0 2cell.store
  0 local.get stack!
next func; make-native 2dup

func: {c-}
  stack@ 4 add stack!
next func; make-native drop

func: {c-}
  stack@ 8 add stack!
next func; make-native 2drop

func: {c-}
  stack@ 0 local.tee
  0 local.get 0 cell.load
  0 local.get
  0 local.get 4 cell.load
  0 cell.store 4 cell.store
next func; make-native swap

func: {c-}
  stack@ 0 local.tee
  0 local.get 0 2cell.load
  0 local.get
  0 local.get 8 2cell.load
  0 double.store 8 double.store
next func; make-native 2swap

func: {c-}
  stack@ 4 sub 0 local.tee
  0 local.get 8 cell.load
  0 cell.store
  0 local.get stack!
next func; make-native over

func: {c-}
  stack@ 8 sub 0 local.tee
  0 local.get 16 2cell.load
  0 double.store
  0 local.get stack!
next func; make-native 2over

func: {c-}
  stack@ 0 local.tee
  0 local.get 0 cell.load \ retrieve value of head
  4 cell.store  \ store in head + 4
  0 local.get 4 add stack!
next func; make-native nip

func: {c-} locals c
  stack@ 4 sub 0 local.tee \ save head - 4
  4 cell.load 1 local.set  \ save [head]
  0 local.get 0 local.get 8 cell.load 4 cell.store \ store [head + 4] in head
  0 local.get 1 local.get 8 cell.store \ store old [head] in head + 4
  0 local.get 1 local.get 0 cell.store \ store old [head] in head - 4
  0 local.get stack! \ and just save the new stack ptr and we're done
next func; make-native tuck

func: {c-}  \ spin your elements round and round
  stack@
  0 local.tee 0 local.get 0 cell.load
  0 local.get 0 local.get 4 cell.load
  0 local.get 0 local.get 8 cell.load
  0 cell.store
  8 cell.store
  4 cell.store
next func; make-native rot

func: {c-}  \ rot rot
  stack@
  0 local.tee 0 local.get 0 cell.load
  0 local.get 0 local.get 4 cell.load
  0 local.get 0 local.get 8 cell.load
  4 cell.store
  0 cell.store
  8 cell.store
next func; make-native -rot

func: {c-}
  (pop) call (rpush) call
next func; make-native >r

func: {c-}
  (rpop) call (push) call
next func; make-native r>

func: {c-}
  rp@ 0 cell.load (push) call
next func; make-native r@

func: {c-}
  RETURN_STACK_BASE i32.const
  rp@ i32.sub
  2 i32.const i32.shr_u
  (push) call
next func; make-native r-depth

: i32-comparator-start ( -- )
  stack@ 0 local.tee
  0 i32.const
  0 local.get 4 cell.load
  0 local.get 0 cell.load
;
: i32-comparator-done ( -- )
  i32.sub
  4 cell.store
  0 local.get 4 add stack!
;

func: {c-}
  i32-comparator-start
  i32.eq
  i32-comparator-done
next func; make-native =

func: {c-}
  i32-comparator-start
  i32.ne
  i32-comparator-done
next func; make-native <>

func: {c-}
  i32-comparator-start
  i32.lt_s
  i32-comparator-done
next func; make-native <

func: {c-}
  i32-comparator-start
  i32.lt_u
  i32-comparator-done
next func; make-native u<

func: {c-}
  i32-comparator-start
  i32.le_s
  i32-comparator-done
next func; make-native <=

func: {c-}
  i32-comparator-start
  i32.le_u
  i32-comparator-done
next func; make-native u<=

func: {c-}
  i32-comparator-start
  i32.gt_s
  i32-comparator-done
next func; make-native >

func: {c-}
  i32-comparator-start
  i32.gt_u
  i32-comparator-done
next func; make-native u>

func: {c-}
  i32-comparator-start
  i32.ge_s
  i32-comparator-done
next func; make-native >=

func: {c-}
  i32-comparator-start
  i32.ge_u
  i32-comparator-done
next func; make-native u>=

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 0 cell.load
  i32.eqz
  i32.sub
  0 cell.store
next func; make-native =0

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 0 cell.load
  0 i32.const i32.ne
  i32.sub
  0 cell.store
next func; make-native <>0

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 0 cell.load
  0 i32.const i32.lt_s
  i32.sub
  0 cell.store
next func; make-native <0

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 0 cell.load
  0 i32.const i32.gt_s
  i32.sub
  0 cell.store
next func; make-native >0

: cc-c-start ( -- )
  stack@ 0 local.tee
  0 local.get 4 cell.load
  0 local.get 0 cell.load
;
: cc-c-done ( -- )
  4 cell.store
  0 local.get 4 add stack!
;

: dd-d-start ( -- )
  stack@ 0 local.tee
  0 local.get 8 double.load
  0 local.get 0 double.load
;

: dd-d-done ( -- )
  8 double.store
  0 local.get 8 add stack!
;

: dc-d-start ( -- )
  stack@ 0 local.tee
  0 local.get 4 double.load
  0 local.get 0 cell.load
;

: dc-d-done ( -- )
  4 double.store
  0 local.get 4 add stack!
;

func: {c-}
  cc-c-start
  i32.add
  cc-c-done
next func; make-native +

func: {c-}
  cc-c-start
  i32.sub
  cc-c-done
next func; make-native -

func: {c-}
  stack@ 0 local.tee
  0 local.get 0 cell.load
  1 add 0 cell.store
next func; make-native 1+

func: {c-}
  stack@ 0 local.tee
  0 local.get 0 cell.load
  1 sub 0 cell.store
next func; make-native 1-

func: {c-}
  cc-c-start
  i32.mul
  cc-c-done
next func; make-native *

func: {c-}
  dd-d-start
  i64.add
  dd-d-done
next func; make-native d+

func: {c-}
  dd-d-start
  i64.sub
  dd-d-done
next func; make-native d-

func: {c-}
  dc-d-start
  i64.extend_i32_s
  i64.mul
  dc-d-done
next func; make-native d*

func: {c-}
  dc-d-start
  i64.extend_i32_u
  i64.mul
  dc-d-done
next func; make-native ud*

func: {c-} locals c
  (pop) call 0 local.tee
  (pop) call 1 local.tee
  0 local.get 1 local.get
  i32.lt_s select
  (push) call
next func; make-native min

func: {c-} locals c
  (pop) call 0 local.tee
  (pop) call 1 local.tee
  0 local.get 1 local.get
  i32.gt_s select
  (push) call
next func; make-native max

func: {c-}
  cc-c-start
  i32.and
  cc-c-done
next func; make-native and

func: {c-}
  cc-c-start
  i32.or
  cc-c-done
next func; make-native or

func: {c-}
  cc-c-start
  i32.shl
  cc-c-done
next func; make-native lshift

func: {c-}
  cc-c-start
  i32.shr_u
  cc-c-done
next func; make-native rshift

func: {c-}
  (pop) call 0 local.tee
  0 local.get 0 cell.load
  (pop) call i32.or
  0 cell.store
next func; make-native cset

func: {c-}
  (pop) call 0 local.tee
  0 local.get 0 cell.load
  (pop) call -1 i32.const i32.xor i32.and
  0 cell.store
next func; make-native creset
