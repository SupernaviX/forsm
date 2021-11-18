1024 constant filebuf-data-size
4 cells filebuf-data-size + constant filebuf-size
: filebuf>fid   0 cells + ;
: filebuf>next  1 cells + ;
: filebuf>head  2 cells + ;
: filebuf>len   3 cells + ;
: filebuf>data  4 cells + ;

: filebuf>contents ( filebuf -- c-addr u )
  dup filebuf>head @ swap filebuf>len @
;


variable filebufs

: filebuf-new ( fid -- err )
  filebuf-size allocate
  dup if nip exit else drop then \ rethrow error
  tuck filebuf>fid !
  filebufs @ over filebuf>next !
  dup filebuf>data over filebuf>head !
  0 over filebuf>len !
  filebufs !
  0
;

: filebuf-delete ( fid -- err )
  >r
  filebufs dup @
  begin dup
  while dup filebuf>fid @ r@ <>
  while nip filebuf>next dup @
  repeat
  then
  r> drop
  dup =0 if drop 0 exit then
  \ found it
  tuck filebuf>next swap ! \ update the holder
  free
;

create iovec 2 cells allot

: filebuf-refill ( filebuf -- err )
  dup >r
  filebuf>len @
    if r> drop 0 exit  \ don't refill if the buffer has any data
    then
  r@ filebuf>data r@ filebuf>head ! \ reset the head
  r@ filebuf>data iovec !
  filebuf-data-size iovec 4 + !
  r@ filebuf>fid @ iovec 1 r> filebuf>len fd-read \ actually read from the file
;

: filebuf-read-line ( target target-len filebuf -- u2 nl? )
  dup >r \ hold onto filebuf
  filebuf>contents
  rot min first-line ( target c-addr u )
  dup >r \ remember how many chars we are reading
  rot swap cmove \ copy into the target buffer
  r> \ return value: how many chars we read
  dup r@ filebuf>contents rot /string \ get the buffer contents with the line removed
  over c@ is-term? over <>0 and -rot \ return value: did we find a newline?
  \ trim leading line terminators
  begin dup
  while over c@ is-term?
  while 1 /string
  repeat
  then
  \ shrink the buffer
  r@ filebuf>len ! r> filebuf>head !
;

: find-filebuf ( fid -- filebuf )
  filebufs @
  begin dup
  while 2dup filebuf>fid @ <>
  while filebuf>next @
  repeat
  then
  nip
;

: open-file ( c-addr u fam -- fid err )
  \ the host has already defined a non-buffering version of this
  open-file
  dup if exit else drop then \ rethrow error
  dup filebuf-new
;

: close-file ( fid -- err )
  dup filebuf-delete
  dup if exit else drop then
  fd-close
;

: read-line ( c-addr u1 fid -- u2 flag err )
  find-filebuf
  dup =0
    if drop -7 exit \ return an error if this file is unbuffered
    then
  over >r \ store original buffer length
  >r
  begin
    r@ filebuf-refill
    dup if nip r> drop r> drop exit else drop then \ rethrow error
    r@ filebuf>len @ \ did we read any data from the file?
    dup =0 if true >r then \ if not, track that we hit file's end
  while
    2dup r@ filebuf-read-line \ read from the buffer into the target
    >r /string r> \ shift the target past the part we read into
    dup if false >r then \ if we found a newline, track that we did not hit file's end
    =0 \ loop if we are still looking for a newline?
  while
  repeat
  then
  ( c-addr u1-u2 ) ( r: u1 filebuf flag )
  nip r> r> drop r> ( u1-u2 flag u1 )
  rot - swap 0  ( u2 flag err )
;
