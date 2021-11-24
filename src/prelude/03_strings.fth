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

\ bake a string into a colon definition
: sliteral ( c-addr u -- )
  >r >r
  postpone ahead
  r> here tuck r@ cmove \ bake in the string
  r@ allot align \ reserve space for the string
  >r
  postpone then
  r> r> swap
  postpone literal postpone literal \ bake in the addr + length
; immediate

create stemp-buffers 320 allot
variable stemp-index
0 stemp-index !

: stemp-buffer ( -- c-addr )
  stemp-buffers stemp-index @ 80 * + \ address of the current buffer
  stemp-index @ 1+ 3 and stemp-index ! \ choose another buffer next time
;

\ store a string in a temporary buffer
: stemp ( c-addr u -- c-addr u )
  dup >r \ store length for later
  stemp-buffer
  dup >r \ store address for later
  swap cmove \ copy to the buffer
  r> r>
;
