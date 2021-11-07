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
  postpone s" postpone type
; immediate