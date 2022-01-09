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
func; make-native execute

func: {c-}
  (pop) call (proc-exit) call
next func; make-native proc-exit

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
  0 2cell.store 8 2cell.store
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
  0 2cell.store
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
  stack@ 0 local.tee
  0 local.get 0 cell.load \ read the head of the stack
  1 add \ +1 to account for the address itself at the top of the stack
  2 i32.const i32.shl \ *4 to make it an offset
  0 local.get i32.add 0 cell.load \ read that offset from the head
  0 cell.store \ and store it back ni the head
next func; make-native pick

func: {c-}
  PARAM_STACK_BASE i32.const
  stack@ i32.sub
  2 i32.const i32.shr_u
  (push) call
next func; make-native depth

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
: i64-comparator-start ( -- )
  stack@ 0 local.tee
  0 i32.const
  0 local.get 8 double.load
  0 local.get 0 double.load
;
: i64-comparator-done ( -- )
  i32.sub
  12 cell.store
  0 local.get 12 add stack!
;

func: {c-}
  i32-comparator-start
  i32.eq
  i32-comparator-done
next func; make-native =

func: {c-}
  i64-comparator-start
  i64.eq
  i64-comparator-done
next func; make-native d=

func: {c-}
  i32-comparator-start
  i32.ne
  i32-comparator-done
next func; make-native <>

func: {c-}
  i64-comparator-start
  i64.ne
  i64-comparator-done
next func; make-native d<>

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
  i64-comparator-start
  i64.lt_s
  i64-comparator-done
next func; make-native d<

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
  i64-comparator-start
  i64.le_s
  i64-comparator-done
next func; make-native d<=

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
  i64-comparator-start
  i64.gt_s
  i64-comparator-done
next func; make-native d>

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
  i64-comparator-start
  i64.ge_s
  i64-comparator-done
next func; make-native d>=

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

: cc-load-locals ( -- )
  0 local.get 4 cell.load 1 local.tee
  0 local.get 0 cell.load 2 local.tee
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

: dd-load-locals ( -- )
  0 local.get 8 double.load 1 local.tee
  0 local.get 0 double.load 2 local.tee
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

: dc-load-locals ( -- )
  0 local.get 4 double.load 1 local.tee
  0 local.get 0 cell.load i64.extend_i32_s 2 local.tee
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
  stack@ 0 local.tee
  0 local.get 0 cell.load
  1 i32.const i32.shl 0 cell.store
next func; make-native 2*

func: {c-}
  stack@ 0 local.tee
  0 local.get 0 cell.load
  1 i32.const i32.shr_s 0 cell.store
next func; make-native 2/

func: {c-}
  stack@ 0 local.tee
  -1 i32.const 0 local.get 0 cell.load i32.xor
  0 cell.store
next func; make-native invert

func: {c-}
  stack@ 0 local.tee
  0 i32.const 0 local.get 0 cell.load i32.sub
  0 cell.store
next func; make-native negate

func: {c-} locals cc
  stack@ 0 local.tee
  0 local.get 0 cell.load 1 local.tee
  31 i32.const i32.shr_s 2 local.tee
  1 local.get i32.xor 2 local.get i32.sub
  0 cell.store
next func; make-native abs

func: {c-}
  stack@ 4 sub 0 local.tee
  0 local.get 4 cell.load i64.extend_i32_s 0 double.store
  0 local.get stack!
next func; make-native s>d

v-' drop v-@ make-native d>s \ converting a double to a cell is just "drop"

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

func: {c-}
  dd-d-start
  i64.mul
  dd-d-done
next func; make-native dd*

func: {c-}
  stack@ 0 local.tee
  0 i64.const 0 local.get 0 double.load i64.sub
  0 double.store
next func; make-native dnegate

func: {c-} locals dd
  stack@ 0 local.tee
  0 local.get 0 double.load 1 local.tee
  63 i64.const i64.shr_s 2 local.tee
  1 local.get i64.xor 2 local.get i64.sub
  0 double.store
next func; make-native dabs

func: {c-} locals dd
  stack@ 0 local.tee dc-load-locals
  i64.rem_u i32.wrap_i64
  8 cell.store
  0 local.get 1 local.get 2 local.get
  i64.div_u i32.wrap_i64
  4 cell.store
  0 local.get 4 add stack!
next func; make-native um/mod

func: {c-} locals ccccc
  stack@ 0 local.tee cc-load-locals
  i32.div_s 3 local.set \ store quotient for now
  1 local.get 2 local.get
  i32.rem_s 4 local.tee \ store remainder as well

  \ use the remainder to find the "real" modulo
  2 local.get 0 i32.const \ conditionally add the divisor
  1 local.get 2 local.get i32.xor 0 i32.const i32.lt_s \ if divisor and dividend have mismatched signs
  4 local.get 0 i32.const i32.ne i32.and \ AND if remainder is nonzero
  5 local.tee \ (store this decision for later)
  select i32.add
  4 cell.store

  \ use the quotient to find the "real" quotient
  0 local.get
  3 local.get 
  1 i32.const 0 i32.const \ conditionally subtract 1
  5 local.get \ with the same conditions as before
  select i32.sub
  0 cell.store
next func; make-native /mod

make-colon /
  v-' /mod v-, v-' nip v-,
v-' exit v-,

make-colon mod
  v-' /mod v-, v-' drop v-,
v-' exit v-,

func: {c-} locals cc
  stack@ 0 local.tee cc-load-locals
  i32.rem_u
  4 cell.store
  0 local.get 1 local.get 2 local.get
  i32.div_u
  0 cell.store
next func; make-native u/mod

func: {c-} locals dd
  stack@ 0 local.tee dc-load-locals
  i64.rem_s i32.wrap_i64
  8 cell.store
  0 local.get 1 local.get 2 local.get
  i64.div_s i32.wrap_i64
  4 cell.store
  0 local.get 4 add stack!
next func; make-native sm/rem

func: {c-} locals ddddc
  stack@ 0 local.tee dc-load-locals
  i64.div_s 3 local.set \ store quotient for now
  1 local.get 2 local.get
  i64.rem_s 4 local.tee \ store remainder as well

  \ use the remainder to find the "real" modulo
  2 local.get 0 i64.const \ conditionally add the divisor
  1 local.get 2 local.get i64.xor 0 i64.const i64.lt_s \ if divisor and dividend have mismatched signs
  4 local.get 0 i64.const i64.ne i32.and \ AND if remainder is nonzero
  5 local.tee \ (store this decision for later)
  select i64.add i32.wrap_i64
  8 cell.store

  \ use the quotient to find the "real" quotient
  0 local.get
  3 local.get 
  1 i64.const 0 i64.const \ conditionally add 1
  5 local.get \ with the same conditions as before
  select i64.sub i32.wrap_i64
  4 cell.store
  0 local.get 4 add stack!
next func; make-native fm/mod

func: {c-} locals dd
  stack@ 0 local.tee dc-load-locals
  i64.rem_u i32.wrap_i64
  8 cell.store
  0 local.get 1 local.get 2 local.get
  i64.div_u
  0 double.store
next func; make-native ud/mod

func: {c-} locals cc
  stack@ 0 local.tee cc-load-locals
  1 local.get 2 local.get
  i32.lt_s select
  cc-c-done
next func; make-native min

func: {c-} locals dd
  stack@ 0 local.tee dd-load-locals
  1 local.get 2 local.get
  i64.lt_s select
  dd-d-done
next func; make-native dmin

func: {c-} locals cc
  stack@ 0 local.tee cc-load-locals
  1 local.get 2 local.get
  i32.gt_s select
  cc-c-done
next func; make-native max

func: {c-} locals dd
  stack@ 0 local.tee dd-load-locals
  1 local.get 2 local.get
  i64.gt_s select
  dd-d-done
next func; make-native dmax

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
  cc-c-start
  i32.shr_s
  cc-c-done
next func; make-native arshift

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
