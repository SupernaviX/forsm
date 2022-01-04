create inputbuf 80 allot
: main ( -- )
  inputbuf 80 accept
  inputbuf over type 10 emit
  abort
;