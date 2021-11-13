: assert-eq ( actual expected -- )
  2dup = if 2drop exit then
  cr ." expected: " . ." actual: " .
  cr ." stack: " .s
  cr bye
;

: assert-0 ( actual -- )
  0 assert-eq
;

\ test erroring when you ask for too damn much memory
13379001401 allocate -3 assert-eq drop
\ test erroring when you double-free
cr .s
1 allocate assert-0
cr .s
dup free assert-0
cr .s
free -3 assert-eq
cr .s

cr \ test allocate and free
cr heap-end @ .
1 allocate assert-0
cr heap-end @ .
1024 allocate assert-0
cr heap-end @ .
1 allocate assert-0
cr heap-end @ .
cr .s
free assert-0
cr heap-end @ .
swap free assert-0
cr heap-end @ .
cr .s
free assert-0
cr .s
cr heap-end @ .

cr \ test resizing at the frontier
cr 8 allocate assert-0 .s
cr dup 4 - @ .
cr 16 resize assert-0 .s
cr dup 4 - @ .
cr 32 resize assert-0 .s
cr dup 4 - @ .
cr 64 resize assert-0 .s
cr dup 4 - @ .
free assert-0
cr heap-end @ .

cr \ test resizing in-place
8 allocate assert-0
1024 allocate assert-0
8 allocate assert-0
swap free assert-0
swap
cr .s \ head of stack is an 8-byte allocation
cr dup 4 - @ .
cr 16 resize assert-0 .s
cr dup 4 - @ .
cr 32 resize assert-0 .s
cr dup 4 - @ .
cr 64 resize assert-0 .s
cr dup 4 - @ .
cr 64 resize assert-0 .s
cr dup 4 - @ .
cr 32 resize assert-0 .s
cr dup 4 - @ .
cr 16 resize assert-0 .s
cr dup 4 - @ .
cr 8 resize assert-0 .s
cr dup 4 - @ .
free assert-0
cr heap-end @ .
free assert-0
cr heap-end @ .
.s

cr \ test reallocation
32 allocate assert-0
32 allocate assert-0
swap
cr .s \ head-of-stack is a 32-byte allocation, exactly big enough to fit
cr heap-end @ .
\ it sure would be a shame if someone... resized it
cr 16 resize assert-0 .s \ OH NO
cr dup 4 - @ .
\ store some data into it
420 over !
69 over 3 cells + !
cr 64 resize assert-0 .s \ FIEND
cr dup 4 - @ .
dup @ 420 assert-eq
dup 3 cells + @ 69 assert-eq
free assert-0
free assert-0
cr heap-end @ .
