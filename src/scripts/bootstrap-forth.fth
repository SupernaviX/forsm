: main ( -- )
  1 2 3 4 .s cr 2drop 2drop .s cr
  refill drop
  begin parse-name dup
  while type cr
  repeat
  2drop
  ." nice" cr
  69 abort
;