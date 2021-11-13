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
heap-start 4 + heap-end !
5 heap-start ! \ start with an empty "block"
5 heap-end @ ! \ end with one too

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
    -2 and +         \ on to the next block
  repeat
  drop r> drop 0
;

\ Reserve a u-sized block at a-aadr with the given occupied flag
\ blocks start and end with their size, plus an occupied flag in the low bit
( block-addr u flag -- )
: reserve-block
  over >r
  + 2dup swap !
  swap r> + 4 - !
;

\ mark a block as used
( block-addr -- )
: use-block
  dup @ 1 reserve-block
;

\ given block dimensions (addr + size), include any preceding free blocks 
( block-addr u -- block-addr u )
: ?merge-before
  over 4 - @ dup 1 and =0
    if tuck + -rot - swap
    else drop
    then
;

\ given block dimensions (addr + size), include any following free blocks 
( block-addr u -- block-addr u )
( block-addr u -- block-addr u )
: ?merge-after
  2dup + @ dup 1 and =0
    if +
    else drop
    then
;

\ reserve a u-sized block at the frontier,
\ allocating more space if needed
( u -- block-addr err )
: frontier-block
  heap-end @                \ allocate at heap-end by default
  dup 4 - @ dup 1 and =0    \ if the final block is free
    if - else drop then     \ incorporate it in the block we're reserving
  2dup + heap-max >         \ bounds check
    if 2drop -3 exit        \ error if we allocate too much
    then
  swap 2dup 1 reserve-block \ new block here
  over + dup heap-end !     \ end of that block is the end of the heap
  5 swap !                  \ empty "block" at the end
  4 + 0                     \ return a-addr pointer and no errors
;

\ Given a free block, make a new used block out of the first u bytes and a new free block out of the rest
( block-addr u -- )
: split-existing-block
  >r
  dup @ r@ - 4 <= \ don't split if the new block would be too smol
    if use-block r> drop exit
    then 
  dup r@ + over @ r@ - 0 reserve-block \ new block at the end of the old one
  r> 1 reserve-block   \ shrink the old one
;

( u block-addr -- a-addr err )
: reuse-existing-block
  tuck swap split-existing-block
  4 + 0
;

( block-addr -- )
: free-block
  dup @ 1-  ( start-addr size )
  ?merge-before
  ?merge-after
  0 reserve-block
;

: allocate ( u -- a-addr err )
  aligned \ make sure the allocation is word-aligned, for performance
  8 +     \ leave room for the header/footer (which should also be word-aligned)
  dup find-free-block
  dup =0
    if drop frontier-block
    else reuse-existing-block
    then
;

: free ( a-addr -- err )
  4 - \ move backwards to the header
  dup c@ 1 and
    if free-block 0 \ if the block is occupied, free it
    else drop -4    \ otherwise you've double-freed, error
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