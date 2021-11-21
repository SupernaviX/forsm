create test-name 80 allot
variable #test-name

: .test-name test-name #test-name @ type ;

variable initial-depth
variable initial-heap-end

: \test ( rest of the line is the test name )
  depth initial-depth !
  heap-end @ initial-heap-end !
  -1 parse 80 min ( c-addr u )
  dup #test-name !
  test-name swap cmove
  cr ." Testing: " .test-name
;

: "assert-eq ( actual expected c-addr u  -- )
  2swap
  2dup = if 2drop 2drop exit then
  cr ." Test failed: " .test-name
  2swap
  dup if cr type else 2drop then
  cr ." expected: " . ." actual: " .
  cr ." stack: " .s
  -1 throw
;

: assert-eq ( actual expected -- )
  0 0 "assert-eq
;

: assert-0 ( actual -- )
  0 assert-eq
;

: \endtest
  depth initial-depth @ s" The stack size has changed. " "assert-eq
  cr ." Test passed: " .test-name
  cr
;

\test erroring when you ask for too damn much memory
1073676288 allocate -3 assert-eq drop
\endtest

\test growing the frontier
65536 allocate assert-0
free assert-0
\endtest

\test erroring when you double-free
1 allocate assert-0
dup free assert-0
free -4 assert-eq
\endtest

\test allocate and free
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
\endtest

\test resizing at the frontier
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
\endtest

\test resizing in-place
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
\endtest

\test reallocation
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
\endtest

bye