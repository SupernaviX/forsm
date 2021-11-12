: allot ( n -- )
  here + cp !
;

: cmove ( c-addr1 c-addr2 u -- )
  0 ?do
    over c@ over c!
    1+ swap 1+ swap
  loop
  2drop
;

: cell-move ( addr1 addr2 u -- )
  2/ 2/ 0 ?do
    over @ over !
    4 + swap 4 + swap
  loop
  2drop
;

61696 constant heap-start
65535 constant heap-max
variable heap-end
heap-start heap-end !

: find-free-block ( u -- a-addr | 0 )
  >r heap-start
  begin
    dup heap-end @ < \ loop while we are not at the end
  while
    dup @
    dup 1 and =0     \ block is not in use
    over r@ >= and   \ block is at least as big as the needful
      if drop r> drop exit
      then
    1 xor +          \ on to the next block
  repeat
  drop r> drop 0
;

( u -- a-addr err )
: create-new-block
  heap-end @            \ hold onto old heap-end, it's the return value
  2dup + heap-max >     \ bounds check
    if 2drop 0 -3 exit   \ error if we allocate too much
    then
  over 1+ over !        \ add a header to heap-end
  swap heap-end +!      \ point heap-end to the new end
  4 + 0                 \ return a pointer AFTER the header, and no errors
;

( u block-addr -- )
: split-existing-block
  2dup @ swap - >r r@ 4 <=
    if r> drop 2drop exit \ don't split if the new block would be too smol
    then
  tuck + r@ swap !  \ create a new block at the end of the old one
  r> negate swap +!  \ shrink this block
;

( block-addr -- a-addr err )
: reuse-existing-block
  1 over +!
  4 + 0
;

: allocate ( u -- a-addr err )
  aligned \ make sure the allocation is word-aligned, for performance
  4 +     \ leave room for the header (which should also be word-aligned)
  dup find-free-block
  dup =0
    if drop create-new-block
    else
      tuck
      split-existing-block
      reuse-existing-block
    then
;

: free  ( a-addr -- err )
  4 - \ move backwards to the header
  dup c@ 1 and
    if -1 swap +! 0  \ if this block is occupied, free it
    else drop -4 \ otherwise you've double-freed and we should return an error
    then  
;

: resize ( a-addr u -- a-addr err )
  allocate
  dup if nip exit else drop then  \ rethrow allocate's error
  2dup \ keep a copy of the old and new addrs on the heap
  over 4 - @ 1-
  over 4 - @ min  \ find the amount to copy ( lesser of old or new size )
  4 -             \ oh and also skip the header
  cell-move
  swap free \ rethrow free's error
;