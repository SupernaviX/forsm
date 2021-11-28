0 constant struct ; \ starting a struct puts a length on the stack
: end-struct ( size --  ) constant ; \ ending a struct creates a word for it

: field ( off1 size -- off2 )
  : over postpone literal postpone + postpone ;
  +
;
