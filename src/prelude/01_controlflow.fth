\ branching!
: >mark here 0 , ;
: >resolve here swap ! ;
: <mark here ;
: <resolve , ;

\ compile-time literals!

: ['] \ ['] DUP pushes the XT of dup onto the stack at runtime
  ' \ get the XT
  [ ' LIT , ' LIT , ] , \ compile LIT
  , \ compile the XT
; immediate

: literal ( n -- ) \ [ 6 ] literal pushes 6 onto the stack at runtime
  ['] LIT , ,
; immediate

\ Conditionals!
: if ['] ?branch , >mark ; immediate
: else ['] branch , >mark swap >resolve ; immediate
: then >resolve ; immediate

\ POSTPONE parses a word, and compiles its compilation semantics into the current word
: POSTPONE ( "ccc" -- )
  parse-name find-name dup =0 if -1 throw then \ Find the nt for the next word, throw if we can't
  dup name>immediate?
    if    name>xt , \ compile this XT into the def
    else  ['] lit , name>xt , ['] , , \ compile "compile this XT" into the def
    then
  ; immediate

\ Loops!
: begin <mark ; immediate
: until POSTPONE ?branch <resolve ; immediate
: again POSTPONE branch <resolve ; immediate
: while POSTPONE ?branch >mark ; immediate
: repeat swap POSTPONE branch <resolve >resolve ; immediate

\ recursion!
: recurse
  last-word @ name>xt ,
; immediate


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
  postpone over postpone over postpone <>
  postpone ?branch >mark do-sys ! \ possible forward branch here
  postpone >r postpone >r
; immediate

\ end of a do loop, increment I and if we HIT the loop end we are done
: loop ( -- )
  postpone r> postpone 1+ postpone r> ( newi target )
  postpone over postpone over postpone = ( newi target ? )
  postpone ?branch <resolve 
  postpone drop postpone drop
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
  postpone drop postpone drop
  do-sys @ >resolve-chain
  do-sys !
; immediate

\ exit the loop early
: leave
  postpone r> postpone r> postpone drop postpone drop
  postpone ?branch >mark-chain
; immediate

: i ( -- n ) postpone r@ ; immediate
