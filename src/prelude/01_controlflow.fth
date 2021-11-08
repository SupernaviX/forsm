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
  false do-sys !          \ no forward branching here
  <mark
  postpone >r postpone >r
; immediate

\ like do, but only run if target ain't == start
: ?do ( target start -- )
  do-sys @
  postpone swap
  postpone 2dup postpone =
  postpone ?branch >mark
  postpone 2drop postpone branch >mark do-sys ! \ possible forward branch here
  >resolve
  <mark                   
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

variable catch-depth

: catch ( xt -- thrown )
  catch-depth @ >r \ store the old catch depth in the return stack
  r-depth catch-depth ! \ store the size of the return stack as the new catch depth
  execute \ run the code
  \ we only execute below this part if all goes well
  r> catch-depth ! \ restore the old catch depth
  0 \ return 0 because nothing went wrong
;
: throw ( err -- )
  dup =0 if
    drop exit \ do nothing if all is well
  then
  \ get the return stack back to the state it was in in "catch"
  begin r-depth catch-depth @ > while r> drop repeat
  \ we are now effectively "inside" catch again
  r> catch-depth ! \ restore the old catch depth
  \ now that we've messed with the return stack, we're actually returning from "catch"
;

\ exceptions!
variable catch-depth

: catch ( xt -- thrown )
  catch-depth @ >r \ store the old catch depth in the return stack
  r-depth catch-depth ! \ store the size of the return stack as the new catch depth
  execute \ run the code
  \ we only execute below this part if all goes well
  r> catch-depth ! \ restore the old catch depth
  0 \ return 0 because nothing went wrong
;
: throw ( err -- )
  dup =0 if
    drop exit \ do nothing if all is well
  then
  \ get the return stack back to the state it was in in "catch"
  begin r-depth catch-depth @ > while r> drop repeat
  \ we are now effectively "inside" catch again
  r> catch-depth ! \ restore the old catch depth
  \ now that we've messed with the return stack, we're actually returning from "catch"
;

