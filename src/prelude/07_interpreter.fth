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

: include-named-file ( name name# fid -- )
  add-file-source
  begin refill
  while
    ['] interpret catch ?dup
      if drop-source throw
      then 
  repeat
  drop-source
;

: include-file ( fid -- )
  0 0 rot include-named-file
;

: save-filename ( c-addr u -- c-addr u )
  tuck here >r
  dup allot align
  r@ swap move
  r> swap
;

: included ( c-addr u -- )
  resolve-relative-path \ make sure the path we save is absolute
  2dup save-filename 2swap
  r/o open-file throw
  include-named-file
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
