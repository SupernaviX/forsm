32 constant bl

\ "adjust" the head of a string. Like a more dangerous substring
: /string ( c-addr1 u1 n -- c-addr2 u2 )
  tuck - -rot + swap
;

\ return the substring of the input starting with c ( if any )
: scan ( c-addr1 u1 c -- c-addr2 u2 )
  >r
  begin dup
  while over c@ r@ <>
  while 1 /string
  repeat
  then
  r> drop
;

\ return the substring of the input after any leading c ( if any )
: remove-start ( c-addr1 u1 c -- c-addr2 u2 )
  >r
  begin dup
  while over c@ r@ =
  while 1 /string
  repeat
  then
  r> drop
;