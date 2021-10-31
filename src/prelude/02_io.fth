32 constant bl
: cr 10 emit ;

\ quick numeric utilities
: pad here 340 + ;
variable holdptr
variable holdend

: <# \ start formatting a number
  pad dup holdptr ! holdend !
;

: #> \ stop formatting a number, return the string
  2drop holdptr @ holdend @ holdptr @ -
;

: hold \ include 1 character
  -1 holdptr +! \ reserve some space
  holdptr @ C! \ and write the char
;

: # \ include one digit
  base @ ud/mod rot
  dup 9 <=
    if 48 +
    else 55 +
    then
  hold
;

: #s \ include all remaining digits
  begin
    #
    2dup or =0
  until
;

: sign \ include a - if the number is negative
  <0 if 45 hold then
;

\ functions to display the top of the stack
: ud. <# #s #> type ;
: d. dup -rot dabs <# #s rot sign #> type ;
: u. 0 ud. ;
: . s>d d. ;