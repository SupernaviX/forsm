32 constant bl

: is-term? ( c -- ? ) dup 10 = swap 13 = or ;

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

\ how many chars at the start of the string match c ?
: prefix-length ( c-addr1 u1 c -- n )
  over >r
  remove-start
  nip r> swap -
;
\ return new string and # of chars consumed
: take-until ( c-addr1 u1 c -- c-addr2 u2 n )
  over >r
  scan
  r> over -
;

\ Return the first line of the input
: first-line ( c-addr1 u1 -- c-addr1 u2 )
  2dup
  begin dup
  while over c@ is-term? =0
  while 1 /string
  repeat
  then
  nip -
;