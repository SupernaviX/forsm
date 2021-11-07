\ branching!
: >mark here 0 , ;
: >resolve here swap ! ;
: <mark here ;
: <resolve , ;

\ Conditionals!
: if postpone ?branch >mark ; immediate
: ahead postpone branch >mark ; immediate
: else postpone branch >mark swap >resolve ; immediate
: then >resolve ; immediate

\ Loops!
: begin <mark ; immediate
: until POSTPONE ?branch <resolve ; immediate
: again POSTPONE branch <resolve ; immediate
: while POSTPONE ?branch >mark ; immediate
: repeat swap POSTPONE branch <resolve >resolve ; immediate

\ do loops!

variable do-sys

: >mark-chain
  do-sys @      \ get old do-sys on the stack
  here do-sys ! \ update do-sys
  ,             \ write new do-sys into the hole
;
: >resolve-chain  ( do-sys -- )
  dup if          
    dup @ swap  ( prev addr )
    here swap ! ( prev )
    recurse     \ the value stored in the recursion hole before is the next place to resolve
  else
    drop        \ addr 0 means the chain is done
  then
;

\ start of a do loop. always runs the body at least once
: do ( target start -- )
  do-sys @
  postpone swap
  <mark
  false do-sys !          \ no forward branching here
  postpone >r postpone >r
; immediate

\ like do, but only run if target ain't == start
: ?do ( target start -- )
  do-sys @
  postpone swap
  <mark                   
  postpone 2dup postpone <>
  postpone ?branch >mark do-sys ! \ possible forward branch here
  postpone >r postpone >r
; immediate

\ end of a do loop, increment I and if we HIT the loop end we are done
: loop ( -- )
  postpone r> postpone 1+ postpone r> ( newi target )
  postpone 2dup postpone = ( newi target ? )
  postpone ?branch <resolve 
  postpone 2drop
  do-sys @ >resolve-chain
  do-sys !
; immediate

\ true if newi JUST crossed the threshold of target
: (+done?) ( oldi newi target )
  tuck < ( oldi target newi<target? )
  -rot < ( newi<target? oldi<target?)
  <>
;

\ loop but iterate by some custom amount, and break if we PASS target
: +loop ( inc -- )
  postpone r> postpone tuck postpone + ( oldi newi )
  postpone tuck postpone r@ ( newi oldi newi target )
  postpone (+done?) postpone r> postpone swap ( newi target ? )
  postpone ?branch <resolve
  postpone 2drop
  do-sys @ >resolve-chain
  do-sys !
; immediate

\ exit the loop early
: leave
  postpone r> postpone r> postpone 2drop
  postpone ?branch >mark-chain
; immediate

: i ( -- n ) postpone r@ ; immediate
