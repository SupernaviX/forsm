: cr ( -- ) 10 emit ;
: space ( -- ) bl emit ;
: spaces ( i -- ) 0 ?do space loop ;

\ quick numeric utilities
: pad here 340 + ;
variable holdptr
variable holdend

: holdlen ( -- u )
  holdend @ holdptr @ -
;

: <# \ start formatting a number
  pad dup holdptr ! holdend !
;

: #> \ stop formatting a number, return the string
  2drop holdptr @ holdlen
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

\ pad with spaces up to a certain length
: paduntil ( u -- )
  holdlen - 0 max 0 ?do
    bl hold
  loop
;

\ words to display numbers
: ud. <# #s #> type space ;
: d. dup -rot dabs <# #s rot sign #> type space ;
: u. 0 ud. ;
: . s>d d. ;

\ words to display right-aligned numbers
: ud.r ( ud width -- )
  >r
  <# #s r> paduntil #> type
;
: d.r ( d width -- )
  >r dup -rot dabs
  <# #s rot sign r> paduntil #> type
;
: u.r ( u width -- )
  swap 0 rot ud.r
;
: .r ( n width -- )
  swap s>d rot d.r
;

: .s \ display the WHOLE stack
  depth
  [char] < emit dup 0 .r [char] > emit space
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