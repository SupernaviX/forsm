: interpret
  begin
    parse-name  \ get the next word
    dup =0 if
      2drop exit \ if it's 0-length, we're done!
    then

    2dup find-name
    ?dup if \ if we found the word in the dictionary,
      nip nip \ get rid of the name
      compiling? if
        compile-name
      else
        interpret-name
      then
    else
      2dup ?number if \ if it's a number, either bake it in or leave it on the stack
        nip nip
        compiling? if
          compile-literal 
        then
      else
        ." Unrecognized word: " type cr
        -14 throw
      then
    then
  again
;

: include-file ( fid -- )
  add-file-source
  begin refill
  while
    ['] interpret catch ?dup
      if drop-source throw
      then 
  repeat
  drop-source
;

: included ( c-addr u -- )
  r/o open-file throw
  include-file
;

: include ( -- ) parse-name included ;

: quit
  begin r-depth while r> drop repeat
  reset-source
  postpone [
  begin refill
  while
    ['] interpret catch ?dup if
      ." Threw exception " . cr
    else
      state @ =0 if space ." ok" cr then
    then
  repeat
  bye
;
