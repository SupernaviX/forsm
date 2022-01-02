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
  (pop) call 0 call
next func; make-native abort

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
  0 local.get 8 double.load 0 double.store
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
  stack@ 4 sub 0 local.tee
  0 local.get 8 cell.load
  0 cell.store
  0 local.get stack!
next func; make-native over

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

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 4 cell.load
  0 local.get 0 cell.load
  i32.eq
  i32.sub
  4 cell.store
  0 local.get 4 add stack!
next func; make-native =

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 4 cell.load
  0 local.get 0 cell.load
  i32.ne
  i32.sub
  4 cell.store
  0 local.get 4 add stack!
next func; make-native <>

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 4 cell.load
  0 local.get 0 cell.load
  i32.lt_s
  i32.sub
  4 cell.store
  0 local.get 4 add stack!
next func; make-native <

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 4 cell.load
  0 local.get 0 cell.load
  i32.lt_u
  i32.sub
  4 cell.store
  0 local.get 4 add stack!
next func; make-native u<

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 4 cell.load
  0 local.get 0 cell.load
  i32.gt_s
  i32.sub
  4 cell.store
  0 local.get 4 add stack!
next func; make-native >

func: {c-}
  stack@ 0 local.tee
  0 i32.const
  0 local.get 4 cell.load
  0 local.get 0 cell.load
  i32.gt_u
  i32.sub
  4 cell.store
  0 local.get 4 add stack!
next func; make-native u>

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
  (pop) call (pop) call
  i32.add
  (push) call
next func; make-native +

func: {c-}
  (pop) call (pop) call
  i32.sub
  (push) call
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
  (pop) call (pop) call
  i32.mul
  (push) call
next func; make-native *

func: {c-}
  (pop) call (pop) call
  i32.and
  (push) call
next func; make-native and

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
