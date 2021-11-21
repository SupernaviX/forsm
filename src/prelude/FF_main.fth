: main
  ." Go forth! Type bye to quit" cr
  here dict-base -
  ." Dictionary size: " dup . ." bytes ("
  100 * dict-capacity / 0 <# #s #> type ." % full)" cr
  quit
;

' main host-finalize