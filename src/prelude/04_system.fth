1024 constant |filebuf.data|
6 cells |filebuf.data| + constant |filebuf|
: filebuf.fid   0 cells + ;
: filebuf.prev  1 cells + ;
: filebuf.next  2 cells + ;
: filebuf.file? 3 cells + ;
: filebuf.head  4 cells + ;
: filebuf.len   5 cells + ;
: filebuf.data  6 cells + ;

\ create a "dummy" filebuf on the stack
create filebufs 3 cells allot \ only include header fields
-1 filebufs filebuf.fid !
filebufs filebufs filebuf.prev !
filebufs filebufs filebuf.next !

: filebuf-new ( fid file? addr -- )
  tuck filebuf.file? !
  tuck filebuf.fid !
  dup filebuf.data over filebuf.head !
  0 over filebuf.len !
  \ link it into the list as the "prev" of the sentinel head
  filebufs over filebuf.next !
  filebufs filebuf.prev @ over filebuf.prev !
  dup filebufs filebuf.prev !
  dup filebuf.prev @ filebuf.next !
;
: filebuf-allot ( fid file? -- )
  here
  |filebuf| allot
  filebuf-new
;
: filebuf-allocate ( fid file? -- err )
  |filebuf| allocate ?dup
    if nip nip
    else filebuf-new 0
    then
;

\ file 0 (stdin) is already open, so add a buffer for it
0 0 filebuf-allot

: filebuf-delete ( filebuf -- err )
  \ link this filebuf's prev and next to each other
  dup filebuf.next @ over filebuf.prev @ filebuf.next !
  dup filebuf.prev @ over filebuf.next @ filebuf.prev !
  free
;

: find-filebuf ( fid -- filebuf | false )
  filebufs filebuf.next @
  begin dup filebufs <>
  while
    2dup filebuf.fid @ =
      if nip exit then
    filebuf.next @
  repeat
  2drop false
;

create iovec 2 cells allot

: filebuf-refill? ( filebuf -- err )
  dup >r
  filebuf.len @
    if r> drop 0 exit  \ don't refill if the buffer has any data
    then
  r@ filebuf.data r@ filebuf.head ! \ reset the head
  r@ filebuf.data iovec !
  |filebuf.data| iovec 4 + !
  r@ filebuf.fid @ iovec 1 r> filebuf.len fd-read \ actually read from the file
;

: filebuf-peek ( filebuf -- char|-1 )
  dup filebuf.len @ =0
    if drop -1
    else filebuf.head @ c@
    then
;

: filebuf-consume ( filebuf -- )
  1 over filebuf.head +!
  -1 swap filebuf.len +!
;

: filebuf-refill-if-file ( filebuf -- err )
  dup filebuf.file? @
    if filebuf-refill?
    else drop 0
    then
;

: is-cr? ( c -- ) 13 = ;
: is-lf? ( c -- ) 10 = ;
: is-term? ( c -- ? ) dup is-cr? swap is-lf? or ;

: filebuf-consume-term ( filebuf -- err )
  >r
  r@ filebuf-refill-if-file
  ?dup if r> drop exit then
  r@ filebuf-peek
  dup is-term? if r@ filebuf-consume then
  is-lf? if r> drop 0 exit then
  \ if we saw an \r, try consuming one more \n
  r@ filebuf-refill-if-file
  ?dup if r> drop exit then
  r@ filebuf-peek is-lf?
    if r> filebuf-consume 0
    else r> drop 0
    then
;

\ options bitmask
1 constant fd-allow-read
2 constant fd-allow-write
4 constant fd-create

: fd-oflags ( options -- oflags )
  fd-allow-write fd-create or and
    if 9 \ creat | trunc
    else 0
    then
;

: fd-rights ( options -- drights )
  >r
  0
  r@ fd-allow-read and
    if 1 or \ fd-read
    then
  r> fd-allow-write and
    if 64 or \ fd-write
    then
  0
;

fd-allow-read constant r/o
fd-allow-write constant w/o

46 constant relative-path-char
47 constant separator-char

create namelengthbuf 2 cells allot
: namelength ( -- u ) namelengthbuf cell + @ ;
\ is this path a child of the parent?
: is-parent-directory? ( path-addr path-u dir-addr dir-u -- ? )
  rot over <= \ if the path length is <= the path length, it can't be a parent
    if 2drop 2drop false exit
    then
  begin ?dup
  while
    -rot
    over c@ over c@ <>
      if drop 2drop false exit
      then
    1+ -rot 1+ -rot 1-
  repeat
  2drop true
;

: normalize-directory-name ( c-addr u -- c-addr u )
  relative-path-char remove-start
  separator-char remove-start
  1- \ remove null terminator
;

variable parent-fd
variable parent-namelength
: get-preopened-relative-path ( c-addr u -- fid c-addr u )
  3 \ this is the first preopened descriptor
  begin
    dup namelengthbuf fd-prestat-get =0
  while
    >r
    namelength allocate throw \ reserve space to hold the name
    r@ over namelength fd-prestat-dir-name throw
    >r 2dup r@ namelength normalize-directory-name is-parent-directory? \ validate whether this is a parent
    r> free throw \ free the name buffer either way
    if
      r@ parent-fd !  \ track that this is a valid parent
      namelength 1- parent-namelength !
    then
    r> 1+ \ try the next
  repeat drop
  parent-namelength @ dup 1 >
    if /string \ remove the parent from the string
    else drop
    then
  separator-char remove-start \ and any leading directory separators
  parent-fd @ -rot \ and return the parent fd AND the pathname
;

variable >fd
: open-fd-by-path ( c-addr u options -- fid err )
  >r \ hold onto options for l8r
  get-preopened-relative-path ( fid path-addr path-u )
  0 -rot ( fid dirflags path-addr path-u )
  r@ fd-oflags r> fd-rights 0 0 0 ( ... oflags drights-base drights-inheriting fdflags )
  >fd path-open
  >fd @ swap ( fid err )
;

\ double-aligned buffer to hold an fdstat
dalign here 8 cells allot constant >fdstat

: is-fd-file? ( fid -- ? err )
  >fdstat fd-fdstat-get
  ?dup if 0 swap exit then \ rethrow error
  >fdstat c@ 4 = 0 \ this is the offset of filetype, and the value of "normal file"
;

: open-file ( c-addr u fam -- fid err )
  dup >r
  open-fd-by-path
  ?dup if r> drop exit then \ rethrow error
  r> fd-allow-read and
    if
      dup is-fd-file?
      ?dup if r> drop nip exit then
      over swap filebuf-allocate
    else 0
    then
;

: create-file ( c-addr u fam -- fid err )
  fd-create or open-file
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
  \ consume one set of trailing terminators
  r> filebuf-consume-term
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

\ command-line arguments
variable argc
0 argc !
variable argv

: init-args ( -- )
  argc @ if exit then
  \ using argv to hold the buffer size temporarily
  argc argv args-sizes-get throw
  \ allot space for both argv and the strings it contains
  here argc @ cells argv @ + aligned allot argv !
  \ populate the args
  argv @ dup argc @ cells + args-get throw
;

: arg ( n -- c-addr u )
  dup argc @ >=
    if drop 0 0 exit then
  cells argv @ + @ ( c-addr )
  \ find null terminator
  dup begin dup c@ while 1+ repeat
  over -
;

: shift-args ( -- )
  argc @ 1 <= if exit then
  argv @ 2 cells + \ copy from argv[2]
  argv @ 1 cells + \ into argv[1]
  argc @ 2 - cells \ copying this many bytes
  move
  -1 argc +!
;

: next-arg ( -- c-addr u )
  1 arg shift-args
;

: bye ( -- ) 0 proc-exit ;