\ case-insensitive name equality
\ assume that c-addr2 is capitalized,
\ and that hidden words have a non-ASCII first character
: name= ( c-addr1 u1 c-addr2 u2 -- ? )
  rot over <> if
    drop 2drop false exit
  then ( c-addr1 c-addr2 len )
  0 ?do ( c-addr1 c-addr2 )
    over c@ upchar over c@ <> if
      2drop false unloop exit
    then
    swap 1+ swap 1+
  loop
  2drop true
;

: find-name ( c-addr u -- nt | 0 )
  latest @ ( c-addr u nt )
  begin dup
  while
    >r 2dup r@ -rot r> \ clone the stack
    name>string name= =0
  while name>backword
  repeat then
  nip nip
;

: ' ( -- xt )
  parse-name find-name
  dup =0 if -2 throw then
  name>xt
;

: ['] ( -- xt )
  ' [ ' literal , ]
; immediate

: postpone ( -- )
  parse-name find-name
  dup =0 if -2 throw then
  dup name>immediate?
    if name>xt ,
    else ['] lit , name>xt , ['] , ,
    then
; immediate

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
        dup name>xt
        swap name>immediate?
          if execute
          else ,
          then
      else
        name>xt execute
      then
    else
      \ TODO: double-width numbers
      2dup s>number? nip if \ if it's a number, either bake it in or leave it on the stack
        nip nip 
        compiling?
          if postpone literal
          then \ no else branch, just leave the number on the stack
      else
        drop
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
