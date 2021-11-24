: cr 10 emit ;
: space bl emit ;

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
    if [char] 0 +
    else [ char A 10 - ] literal +
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
  <0 if [char] - hold then
;

\ words to display numbers
: ud. <# #s #> type space ;
: d. dup -rot dabs <# #s rot sign #> type space ;
: u. 0 ud. ;
: . s>d d. ;

: .s \ display the WHOLE stack
  depth
  [char] < emit dup 0 <# #s #> type [char] > emit space
  dup 0 ?do
    dup i - pick .
  loop
  drop
;

: ." \ display an string
  [char] " parse 
  compiling?
    if postpone sliteral postpone type
    else type
    then
; immediate