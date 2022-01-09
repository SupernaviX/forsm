: parse-args
  init-args
  begin next-arg 2dup 0 0 d<>
  while
    2dup 2>r
    ['] included catch ?dup if
      ." Error " . ." thrown from " 2r> type cr
      bye
    else 2r> 2drop
    then
  repeat
  2drop
;

: main
  parse-args
  ." Go forth! Type bye to quit" cr
  here dict-base -
  ." Dictionary size: " dup . ." bytes ("
  100 * dict-capacity / 0 u.r ." % full)" cr
  quit
;

' main host-finalize