32 constant bl

: uppercase? ( c -- ? ) 65 91 within ; \ ascii 'A' to 'Z'
: lowercase? ( c -- ? ) 97 123 within ; \ asci 'a' to 'z'
: numeric? ( c -- ? ) 48 58 within ; \ ascii '0' to '9'
: upcase ( c -- C ) dup lowercase? if 32 - then ;

\ convert a length-prefixed string to a "normal" string
: count ( str -- c-addr u )
  dup 1+ swap c@
;

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

: str= ( c-addr1 u1 c-addr2 u2 -- ? )
  rot over <>
    if drop 2drop false exit then
  begin
    ?dup =0
      if 2drop true exit then
    -rot over c@ over c@ <>
      if 2drop drop false exit then
    swap 1+ swap 1+ rot 1-
  again
;

\ given a string, return the parts of it before and after the first instance of a char
: split ( c-addr u c -- after-addr after-u before-addr before-u )
  >r 2dup r> scan
  dup >r dup 1 min /string
  2swap r> -
;

variable term
variable #term
: search ( c-addr1 u1 c-addr2 u2 -- c-addr3 u3 flag )
  #term ! term !
  2dup
  begin
    term @ c@ scan
    dup #term @ <
      if 2drop false exit
      then
    over #term @ term @ over str=
      if 2swap 2drop true exit
      then
    1 /string
  again
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
