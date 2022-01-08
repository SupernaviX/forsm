\ compilation utilities
: v-xt ( -- ) v-' v-, ;
: v-lit ( n -- ) [v-'] lit v-, v-, ;

0 make-variable state
make-colon compiling?
  v-xt state v-xt @
v-xt exit

0 make-variable >in
TIB_BASE make-constant tib
0 make-variable #tib

0 make-constant false
-1 make-constant true
4 make-constant cell

DICT_BASE make-constant dict-base
DICT_CAPACITY make-constant dict-capacity
HEAP_BASE make-constant heap-base

(dovar) make-constant (dovar)
(docon) make-constant (docon)
(docol) make-constant (docol)
(dodoes) make-constant (dodoes)

make-colon here
  v-xt cp
  v-xt @
v-xt exit
make-colon ,
  v-xt here v-xt !          \ here !
  v-xt cell v-xt cp v-xt +! \ cell cp +!
v-xt exit

make-colon c,
  v-xt here v-xt c!         \ here c!
  1 v-lit v-xt cp v-xt +!   \ 1 cp +!
v-xt exit

make-colon aligned
  3 v-lit v-xt + -4 v-lit v-xt and \ 3 + -4 and
v-xt exit

make-colon align
  v-xt here v-xt aligned v-xt cp v-xt !
v-xt exit

make-colon upchar
  v-xt dup char a v-lit v-xt >=
  v-xt over char z v-lit v-xt <= v-xt and
  v-xt ?branch v-here 0 v-, \ if
  32 v-lit v-xt -
  v-here swap v-! \ then
v-xt exit

make-colon header
  v-xt here v-xt >r \ here >r
  v-xt dup v-xt c,  \ dup c,
  v-here \ start of loop ( holding this address on the stack )
    v-xt dup v-xt ?branch v-here 0 v-,  \ dup ?branch [after loop]
    v-xt swap v-xt dup v-xt c@ v-xt upchar v-xt c, \ swap dup c@ upchar c,
    v-xt 1+ v-xt swap v-xt 1- \ 1+ swap 1-
    v-xt branch swap v-, \ branch [start of loop]
  v-here swap v-! \ end of loop
  v-xt 2drop v-xt align \ 2drop align
  v-xt latest v-xt @ v-xt , \ latest @ ,
  v-xt r> v-xt latest v-xt !  \ r> latest !
  v-xt (dovar) v-xt , \ (dovar) ,
v-xt exit

make-colon name>u
  v-xt c@ 31 v-lit v-xt and
v-xt exit

make-colon name>string
  v-xt dup v-xt 1+ v-xt swap v-xt name>u
v-xt exit

make-colon name>backword
  v-xt dup v-xt name>u v-xt 1+ v-xt aligned v-xt + v-xt @
v-xt exit

make-colon name>xt
  v-xt dup v-xt name>u v-xt 1+ v-xt aligned v-xt + v-xt cell v-xt +
v-xt exit

make-colon name>immediate?
  v-xt c@ 64 v-lit v-xt and v-xt <>0
v-xt exit

make-colon xt,
  v-xt latest v-xt @ v-xt name>xt v-xt !
v-xt exit

make-colon immediate
  64 v-lit
  v-xt latest v-xt @
  v-xt cset
v-xt exit

make-colon hide
  32768 v-lit
  v-xt latest v-xt @
  v-xt cset
v-xt exit

make-colon reveal
  32768 v-lit
  v-xt latest v-xt @
  v-xt creset
v-xt exit

make-colon [
  0 v-lit v-xt state v-xt !
v-xt exit
v-immediate

make-colon ]
  -1 v-lit v-xt state v-xt !
v-xt exit

make-colon ;
  v-' exit v-lit v-xt ,
  v-xt reveal
  v-xt [
v-xt exit
v-immediate

make-colon >body
  v-xt cell v-xt +
v-xt exit

make-colon does>
  v-' lit v-lit v-xt , \ compile "lit"
  v-xt here 0 v-lit v-xt , \ compile 0 (but track the address to fill in later)
  v-' xt, v-lit v-xt , \ compile "xt,"
  v-' exit v-lit v-xt , \ compile "exit"
  \ now HERE is at the address of the runtime word, so we can fill in that address from before
  v-xt (dodoes) v-xt here 8 v-lit v-xt lshift v-xt or v-xt swap v-xt !
v-xt exit
v-immediate

make-colon literal
  v-' lit v-lit v-xt ,
  v-xt ,
v-xt exit
v-immediate

make-colon recurse
  v-xt latest v-xt @ v-xt name>xt v-xt ,
v-xt exit
v-immediate