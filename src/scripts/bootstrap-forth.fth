: main ( -- )
  refill drop
  begin parse-name dup
  while type 10 emit
  repeat
  2drop
  s" nice" type 10 emit
  69 abort
;