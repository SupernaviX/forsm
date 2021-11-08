: quit
  r-clear
  0 >source-id !
  0 >in !
  postpone [
  begin
    refill
  while
    interpret
    space ." ok" cr
  repeat
  bye
;

: main
  ." Go forth! Type bye to quit" cr
  quit
;

main