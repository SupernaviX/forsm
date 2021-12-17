0 constant struct ; \ starting a struct puts a length on the stack
: end-struct ( size --  )
  create ,
  does> @
; \ ending a struct creates a word for it

: field ( off1 size -- off2 )
  create over , +
  does> @ +
;

: noop ( -- ) ;
: defer ( -- )
  create ['] noop ,
  does> ( -- ) @ execute 
;
: defer! ( xt defer-xt -- )
  >body !
;
: is ( xt -- )
  compiling? if
    postpone ['] postpone defer!
  else
    ' defer!
  then
; immediate