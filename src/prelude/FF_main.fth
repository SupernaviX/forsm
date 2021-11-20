: quit
  begin r-depth while r> drop repeat
  0 >source-id !
  0 >in !
  postpone [
  begin
    refill
  while
    ['] interpret catch
    ?dup if
      ." Threw exception " . cr
    else
      state @ =0 if space ." ok" cr then
    then
  repeat
  bye
;

: main
  ." Go forth! Type bye to quit" cr
  here dict-base -
  ." Dictionary size: " dup . ." bytes ("
  100 * dict-capacity / 0 <# #s #> type ." % full)" cr
  quit
;

' main host-finalize