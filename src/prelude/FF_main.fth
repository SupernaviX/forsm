: parse-args
  init-args
  begin next-arg 2dup 0 0 d<>
  while included
  repeat
  2drop
;

: main
  decimal
  parse-args
  ." Go forth! Type bye to quit" cr
  here dict-base -
  ." Dictionary size: " dup . ." bytes ("
  100 * dict-capacity / 0 u.r ." % full)" cr
  quit
;

' main host-finalize