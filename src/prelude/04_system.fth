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

: filebuf-new ( fid addr -- )
  tuck filebuf>fid !
  dup filebuf>data over filebuf>head !
  0 over filebuf>len !
  \ link it into the list as the "prev" of the sentinel head
  filebufs over filebuf>next !
  filebufs filebuf>prev @ over filebuf>prev !
  dup filebufs filebuf>prev !
  dup filebuf>prev @ filebuf>next !
;
: filebuf-allot ( fid -- )
  here
  filebuf-size allot
  filebuf-new
;
: filebuf-allocate ( fid -- err )
  filebuf-size allocate ?dup
    if nip nip
    else filebuf-new 0
    then
;

\ file 0 (stdin) is already open, so add a buffer for it
0 filebuf-allot

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

-1 constant r/o
0 constant w/o

: open-file ( c-addr u fam -- fid err )
  \ the host has already defined a non-buffering version of this
  dup >r
  open-file
  ?dup if r> drop exit then \ rethrow error
  r>
    if dup filebuf-allocate
    else 0
    then
;

: close-file ( fid -- err )
  dup find-filebuf ?dup
    if filebuf-delete ?dup
      if nip exit
      then
    then
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

create ciovec 2 cells allot
variable >bytes-written
: write-file ( c-addr u fid -- err )
  rot ciovec ! swap ( fid u )
  begin ?dup
  while
    dup ciovec 4 + ! \ save how many bytes to write
    over ciovec 1 >bytes-written fd-write \ write bytes
    ?dup if nip nip exit then \ rethrow error
    >bytes-written @
    dup ciovec +! \ however many bytes we wrote, move that far forward in the buffer
    - \ and write that many fewer bytes next iteration
  repeat
  drop 0
;

variable emit-buffer
: emit-file ( c fid -- err )
  swap emit-buffer !
  emit-buffer 1 rot write-file
;

: write-line ( c-addr u fid -- err )
  dup >r
  write-file ?dup =0
    if 13 r> emit-file
    else r> drop
    then
;

: accept ( c-addr u1 -- u2 )
  0 read-line throw drop
;
: emit ( c -- ) 1 emit-file throw ;
: type ( c-addr u -- ) 1 write-file throw ;