: interpret
  begin
    parse-name  \ get the next word
    dup =0 if
      2drop exit \ if it's 0-length, we're done!
    then

    2dup find-name
    dup <>0 if \ if we found the word in the dictionary,
      nip nip \ get rid of the name
      compiling? if
        compile-name
      else
        interpret-name
      then
    else
      drop
      ?number if \ if it's a number, either bake it in or leave it on the stack
        compiling? if
          compile-literal 
        then
      else
        -14 throw
      then
    then
  again
;