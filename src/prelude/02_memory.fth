: allot ( n -- )  cp +! ;
4 constant cell
: cells ( n -- n )  2* 2* ;

\ copy start-to-end
: cmove ( c-addr1 c-addr2 u -- )
  0 ?do
    over c@ over c!
    1+ swap 1+ swap
  loop
  2drop
;

\ copy end-to-start
: cmove> ( c-addr1 c-addr2 u -- )
  tuck 2swap + 1- -rot tuck + 1- swap \ move buffers to the end
  0 ?do
    over c@ over c!
    1- swap 1- swap
  loop
  2drop
;

\ copy non-propagatingly
: move ( c-addr1 c-addr2 u )
  >r 2dup >= r> swap
    if cmove
    else cmove>
    then
;

: heap-limit ( -- addr ) memory.size 16 lshift 1 - ;
: grow-heap-if-needed ( target-max -- failed? )
  65535 + 16 rshift memory.size - \ find the number of pages to request
  dup >0
    if memory.grow -1 = \ wasm returns -1 on failure
    else drop 0
    then
;

variable heap-end
heap-base 4 + heap-end !
5 heap-base !  \ start with an empty "block"
7 heap-end @ ! \ end with an empty "heap end" block

: find-free-block ( u -- a-addr | 0 )
  >r heap-base
  begin
    dup heap-end @ < \ loop while we are not at the end
  while
    dup @
    dup 1 and =0     \ block is not in use
    over r@ >= and   \ block is at least as big as the needful
      if drop r> drop exit
      then
    -4 and +         \ on to the next block
  repeat
  drop r> drop 0
;

\ Reserve a u-sized block at a-aadr with the given occupied flag
\ blocks start and end with their size, plus flags in the low bits
( block-addr u flags -- )
: reserve-block
  over >r
  + 2dup swap !
  swap r> + 4 - !
;

: block>used? ( block-addr -- ? ) c@ 1 and ;
: block>end? ( block-addr -- ? )  c@ 2 and ;
: block>size ( block-addr -- u )  @ -4 and ;
: block.next ( block-addr -- block-addr ) dup block>size + ;

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
: ?merge-after
  2dup + @ dup 1 and =0
    if +
    else drop
    then
;

( address -- )
: set-heap-end
  dup heap-end !
  7 swap !
;

\ (try to) shrink or grow the heap
\ returns the old heap-end, and a did-we-fail bool
( u -- block-addr failed? )
: move-heap-end
  heap-end @ tuck +     ( block-addr block-end )
  dup grow-heap-if-needed
    if drop -1
    else set-heap-end 0
    then
;

\ reserve a u-sized block at the frontier
( u -- block-addr failed? )
: frontier-block
  dup move-heap-end         \ try to allocate space
    if nip -1 exit          \ error if we can't
    then
  tuck swap 1 reserve-block \ new used block here
  0                         \ no errors
;

\ Given a free block, make a new used block out of the first u bytes and a new free block out of the rest
( block-addr u -- )
: split-existing-block
  >r
  dup r@ + over @ r@ - 0 reserve-block \ new block at the end of the old one
  r> 1 reserve-block   \ shrink the old one
;

( u -- block-addr failed? )
: allocate-block
  dup find-free-block
  ?dup =0
    if frontier-block
    else tuck swap split-existing-block 0
    then
;

( block-addr -- )
: free-block
  dup @ 1-  ( start-addr size )
  ?merge-before
  ?merge-after
  2dup + block>end? \ if the block after this is the heap end
    if drop set-heap-end \ this is the heap end
    else 0 reserve-block \ this is just a free block
    then
;

: freeable? ( block-addr -- ? )
  dup block>used? <>0
  over heap-base > and
  swap heap-end @ < and
;

: is-frontier? ( block-addr -- ? )
  block.next block>end?
;

: resize-frontier ( block-addr u -- a-addr err )
  2dup swap block>size - move-heap-end nip
    if drop -3 throw exit   \ error if not enough space
    then
  over swap 1 reserve-block
  4 + 0
;

: can-resize-inplace? ( block-addr u -- ? )
  over block.next dup block>used? =0
    if block>size - \ if the next block is free, we need less space
    else drop
    then
  swap block>size <=
;

: resize-inplace ( block-addr u -- a-addr err )
  >r dup
  dup block>size ?merge-after ( block-addr block-addr size )
  swap r@ + swap r@ - \ free the latter section
    dup if 0 reserve-block
    else 2drop  \ (don't free if it's empty of course)
    then
  dup r> 1 reserve-block  \ use the former section
  4 + 0
;

: resize-reallocate ( block-addr u -- a-addr err )
  over block>size over min 8 - >r \ remember how many bytes to copy
  allocate-block
    if \ if allocation failed, restore a known state
      r> 2drop \ clean up the stack
      4 + -3    \ return a pointer to the OG block, plus an error
    else
      4 +
      over 4 + over r> cmove \ copy old contents into new pointer
      swap free-block \ free the OG block now that we are done with it
      0     \ return a pointer to the new block, plus no error
    then
;

\ Allocate a u-sized block of memory on the heap
: allocate ( u -- a-addr err )
  aligned \ make sure the allocation is word-aligned, for performance
  8 +     \ leave room for the header/footer (which should also be word-aligned)
  allocate-block =0
    if 4 + 0  \ return a pointer past the header, and success
    else -3   \ couldn't allocate, return an error
    then
;

\ Free some memory previously allocated on the heap
: free ( a-addr -- err )
  4 - \ move backwards to the header
  dup freeable?
    if free-block 0 \ if the block is occupied, free it
    else drop -4    \ otherwise you've double-freed, error
    then
;

\ Change the bounds of some previously-allocated memory
: resize ( a-addr u -- a-addr err )
  swap 4 - \ look at head of block
  dup freeable? =0
    if drop -4 exit \ can't resize what you can't free
    then
  swap aligned 8 + ( block-addr u )
  over is-frontier?
    if resize-frontier
    else 2dup can-resize-inplace?
      if resize-inplace
      else resize-reallocate
      then
    then
;
