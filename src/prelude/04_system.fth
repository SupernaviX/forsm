1024 constant filebuf-data-size
5 cells filebuf-data-size + constant filebuf-size
: filebuf>fid   0 cells + ;
: filebuf>prev  1 cells + ;
: filebuf>next  2 cells + ;
: filebuf>head  3 cells + ;
: filebuf>len   4 cells + ;
: filebuf>data  5 cells + ;

\ create a "dummy" filebuf on the stack
create filebufs 3 cells allot \ only include header fields
-1 filebufs filebuf>fid !
filebufs filebufs filebuf>prev !
filebufs filebufs filebuf>next !

: filebuf-new ( fid -- err )
  filebuf-size allocate
  ?dup if nip exit then \ rethrow error
  tuck filebuf>fid !
  dup filebuf>data over filebuf>head !
  0 over filebuf>len !
  \ link it into the list as the "prev" of the sentinel head
  filebufs over filebuf>next !
  filebufs filebuf>prev @ over filebuf>prev !
  dup filebufs filebuf>prev !
  dup filebuf>prev @ filebuf>next !
  0
;

\ file 0 (stdin) is already open, so add a buffer for it
0 filebuf-new throw

: filebuf-delete ( filebuf -- err )
  \ link this filebuf's prev and next to each other
  dup filebuf>next @ over filebuf>prev @ filebuf>next !
  dup filebuf>prev @ over filebuf>next @ filebuf>prev !
  free
;

: find-filebuf ( fid -- filebuf | false )
  filebufs filebuf>next @
  begin dup filebufs <>
  while
    2dup filebuf>fid @ =
      if nip exit then
    filebuf>next @
  repeat
  2drop false
;

create iovec 2 cells allot

: filebuf-refill? ( filebuf -- err )
  dup >r
  filebuf>len @
    if r> drop 0 exit  \ don't refill if the buffer has any data
    then
  r@ filebuf>data r@ filebuf>head ! \ reset the head
  r@ filebuf>data iovec !
  filebuf-data-size iovec 4 + !
  r@ filebuf>fid @ iovec 1 r> filebuf>len fd-read \ actually read from the file
;

: filebuf-peek ( filebuf -- char|-1 )
  dup filebuf>len @ =0
    if drop -1
    else filebuf>head @ c@
    then
;

: filebuf-consume ( filebuf -- )
  1 over filebuf>head +!
  -1 swap filebuf>len +!
;

: open-file ( c-addr u fam -- fid err )
  \ the host has already defined a non-buffering version of this
  open-file
  ?dup if exit then \ rethrow error
  dup filebuf-new
;

: close-file ( fid -- err )
  dup filebuf-delete
  ?dup if exit then
  fd-close
;

: read-line ( c-addr u1 fid -- u2 more? err )
  find-filebuf
  ?dup =0 if -7 exit then  \ return an error if this file is unbuffered
  >r tuck \ store filebuf and OG length for later
  begin \ copy while we gotta
    r@ filebuf-refill?
    ?dup if r> drop exit then \ rethrow error
    r@ filebuf-peek
    over \ while we are still reading to the buffer
    over -1 <> and \ and the last char wasn't EOF
    over is-term? =0 and \ and we haven't found a line terminator
  while
    r@ filebuf-consume ( u1 c-addr u c )
    rot tuck c! swap \ write to the buffer ( u1 c-addr u )
    1 /string
  repeat ( u1 c-addr u2 last-char )
  rot drop -rot - ( last-char u )
  swap is-term? over <>0 or ( u more? )
  \ discard newlines
  begin r@ filebuf-peek is-term?
  while r@ filebuf-consume
  repeat
  r> drop 0
;

: accept ( c-addr u1 -- u2 )
  0 read-line throw drop
;
