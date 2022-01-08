: vstr>str ( vc-addr u -- c-addr u ) swap vaddr>addr swap ;

\ Support emulation of words provided by the host
create host-words 32 2* cells allot
variable host-words#
0 host-words# !
: map-host-word ( from-xt to-xt -- )
  host-words# @ 2* cells host-words + tuck ( from-xt addr to-xt addr )
  cell + ! !
  1 host-words# +!
;
: mapped-host-word ( from-xt -- to-xt | false )
  host-words# @ 0 ?do
    i 2* cells host-words +
    2dup @ = if
      nip cell + @
      unloop exit
    else drop
    then
  loop
  drop false
;
: host-execute ( xt -- ran? )
  mapped-host-word ?dup
    if execute true
    else false
    then
;

\ Track execution tokens from the emulated forth which we should NOT emulate.
\ This is for words like "source" which we are using while redefining them.
\ We will compile references to the final definition, but execute the OG one.
create host-deferred-words 16 2* cells allot
variable host-deferred-words#
0 host-deferred-words# !

: host-deferred ( -- )
  >latest v-@ ( nt )
  host-deferred-words host-deferred-words# @ 2* cells + ( nt address )
  over v-name>xt over ! \ store the virtual XT of the deferred word
  swap v-name>string vstr>str 2dup find-name ( address c-addr u real-xt )
  ?dup =0 if
    ." Cannot defer unrecognized word " type cr
    21 throw
  then
  name>xt mapped-host-word ( address c-addr u xt )
  ?dup =0 if
    ." No runtime behavior defined for deferred word " type cr
    22 throw
  then
  nip nip
  swap cell + ! \ store the XT of the word to actually run
  1 host-deferred-words# +!
;

: execute-deferred-word ( xt -- ran? )
  host-deferred-words# @ 0 ?do
    i 2* cells host-deferred-words +
    2dup @ = if
      nip cell + @ execute
      true unloop exit
    else drop
    then
  loop
  drop false
;

\ Use a virtual IP and return stack to emulate words during compilation
variable v-ip
create v-rstack 256 allot
here constant v-r0
variable v-rp
v-r0 v-rp !
: v->r ( value -- ) -4 v-rp +! v-rp @ ! ;
: v-r> ( -- value ) v-rp @ @ 4 v-rp +! ;
: v-r@ ( -- value ) v-rp @ @ ;
: v-rdepth ( -- u ) v-r0 v-rp @ - 2/ 2/ ;

: callable' ( -- callable )
  ['] lit , v-' v-@ ,
; immediate

\ Given an XT from the virtual interpreter, run it
: v-execute' ( v-xt -- )
  >r
  r@ execute-deferred-word if
    cell v-ip +!
    r> drop exit
  then
  r@ v-@
  case
    (docol) of
      v-ip @ v->r \ store current ip on the return stack
      r@ cell + v-ip ! \ new ip is the colon definition
    endof
    callable' exit of
      v-r> cell + v-ip ! \ pop return stack + 4 into ip
    endof
    callable' lit of
      v-ip @ cell + v-@ \ read literal from next cell
      2 cells v-ip +! \ jump past it
    endof
    callable' branch of
      v-ip @ cell + v-@ v-ip !
    endof
    callable' ?branch of =0
      if v-ip @ cell + v-@ v-ip !
      else 2 cells v-ip +!
      then
    endof
    cell v-ip +!  \ everything below this line just increments IP normally
    (dovar) of r@ cell + endof
    (docon) of r@ cell + v-@ endof
    callable' @ of v-@ endof
    callable' c@ of v-c@ endof
    callable' ! of v-! endof
    callable' c! of v-c! endof
    callable' +! of v-+! endof
    callable' cells of cells endof
    callable' cset of v-cset endof
    callable' creset of v-creset endof
    callable' >r of v->r endof
    callable' r> of v-r> endof
    callable' r@ of v-r@ endof
    callable' r-depth of v-rdepth endof
    callable' dup of dup endof
    callable' ?dup of ?dup endof
    callable' 2dup of 2dup endof
    callable' drop of drop endof
    callable' 2drop of 2drop endof
    callable' swap of swap endof
    callable' 2swap of 2swap endof
    callable' over of over endof
    callable' 2over of 2over endof
    callable' nip of nip endof
    callable' tuck of tuck endof
    callable' rot of rot endof
    callable' -rot of -rot endof
    callable' = of = endof
    callable' <> of <> endof
    callable' < of < endof
    callable' u< of u< endof
    callable' <= of <= endof
    callable' u<= of u<= endof
    callable' > of > endof
    callable' u> of u> endof
    callable' >= of >= endof
    callable' u>= of u>= endof
    callable' =0 of =0 endof
    callable' <>0 of <>0 endof
    callable' min of min endof
    callable' max of max endof
    callable' + of + endof
    callable' - of - endof
    callable' 1+ of 1+ endof
    callable' 1- of 1- endof
    callable' * of * endof
    callable' and of and endof
    callable' or of or endof
    callable' lshift of lshift endof
    callable' rshift of rshift endof
    ( default )
      ." Callable not supported: " dup . cr
      140 throw
  endcase
  r> drop
;
\ Given an XT from the virtual interpreter, run it
: v-execute ( v-xt -- )
  v-execute' \ execute the first XT
  begin v-rdepth \ if it was a colon definition,
  while v-ip @ v-@ v-execute' \ keep executing until it's done
  repeat
;

: v-compiling? ( -- ? ) [v-'] state v-execute' v-@ ;
: v-unrecognized-word ( c-addr u -- )
  ." Unrecognized word: " type cr
  -14 throw
;
: v-tried-compiling-host-word ( c-addr u -- )
  ." Cannot compile host word: " type cr
  -15 throw
;
: v-cannot-execute-host-word ( c-addr u -- )
  ." Cannot execute host word: " type cr
  -16 throw
;

\ Given a string, evaluate it through the firtual interpreter
: v-evaluate ( c-addr u -- )
  2dup v-find-name ?dup if
    nip nip
    \ deal with virtual XT
    v-compiling? if
      dup v-name>xt
      swap v-name>immediate?
        if v-execute
        else v-,
        then
    else v-name>xt v-execute
    then
    exit
  then
  2dup find-name ?dup if
    \ deal with host XT
    v-compiling? if
      dup name>immediate? if
        -rot 2>r name>xt host-execute
          if 2r> 2drop
          else 2r> v-cannot-execute-host-word
          then
      else drop v-tried-compiling-host-word
      then
    else
      -rot 2>r name>xt host-execute
        if 2r> 2drop
        else 2r> v-cannot-execute-host-word
        then
    then
  else
    \ maybe this is a number
    2dup s>number? nip if
      nip nip
      v-compiling? if
        [v-'] lit v-, v-,
      then \ no else because the number is already on the stack
    else
      drop
      v-unrecognized-word
    then
  then
;

variable v-source-fid

: v-source-id ( -- n ) v-source-fid @ ;

: v-source ( -- c-addr u )
  [v-body] tib v-@
  [v-body] #tib v-@
;
: v-refill ( -- ? )
  0 [v-body] >in v-! \ reset >IN
  TIB_BASE vaddr>addr TIB_CAPACITY v-source-fid @ ( c-addr u1 fid )
  read-line throw ( u2 more? )
  swap [v-body] #tib v-! \ write how much we read
;

: v-parse-area ( -- vc-addr u ) v-source [v-body] >in v-@ /string ;
: v-parse-consume ( u -- ) [v-body] >in v-+! ;
: v-parse ( c -- vc-addr u )
  >r
  v-parse-area over swap ( ret-addr vc-addr u )
  begin dup \ parse until we see the delimiter or exhaust the string
  while over v-c@ r@ <>
  while 1 /string
  repeat then ( ret-addr vc-addr u )
  >r 2dup swap - swap r> ( ret-addr ret-u vc-addr u )
  begin dup \ remove remaining trailing characters
  while over v-c@ r@ =
  while 1 /string
  repeat then
  r> 2drop \ we are done with the delimiter and remaining string length
  rot tuck - v-parse-consume swap
;
: v-parse-name ( -- vc-addr u )
  v-parse-area over swap ( vc-addr vc-addr u )
  begin dup
  while over v-c@ bl =
  while 1 /string
  repeat then
  drop swap - v-parse-consume
  bl v-parse
;

: v-interpret ( -- )
  begin
    v-parse-name vstr>str
    dup =0 if
      2drop exit
    then
    v-evaluate
    again
;

: v-bootstrap ( c-addr u -- )
  r/o open-file throw v-source-fid ! \ open da file
  begin v-refill
  while v-interpret
  repeat
  v-source-fid @ close-file throw
  0 v-source-fid !
;

\ support calling some debugging utils in the emulator
: v-type ( vc-addr u -- ) vstr>str type ;

\ compilation functions
: v-\ ( -- ) -1 v-parse 2drop ;
: v-( ( -- ) [char] ) v-parse 2drop ;
: v-create ( -- )
  v-parse-name [v-'] header v-execute
;
: v-variable ( -- )
  v-create
  0 v-,
;
: v-constant ( value -- )
  v-create
  [v-'] (docon) v-execute [v-'] xt, v-execute
  v-,
;
: v-: ( -- )
  v-create
  [v-'] (docol) v-execute [v-'] xt, v-execute
  [v-'] hide v-execute
  [v-'] ] v-execute
;
: v-postpone ( -- )
  v-parse-name vstr>str
  2dup v-find-name ?dup if
    nip nip
    dup v-name>immediate?
      if v-name>xt v-,
      else [v-'] lit v-, v-name>xt v-, [v-'] , v-,
      then
  else
    ." Cannot postpone word " type cr
    -19 throw
  then
;


' source-id ' v-source-id map-host-word
' source ' v-source map-host-word
' refill ' v-refill map-host-word
' parse ' v-parse map-host-word
' parse-name ' v-parse-name map-host-word
' \ ' v-\ map-host-word
' ( ' v-( map-host-word
' postpone ' v-postpone map-host-word
' create ' v-create map-host-word
' variable ' v-variable map-host-word
' constant ' v-constant map-host-word
' : ' v-: map-host-word
' host-deferred ' host-deferred map-host-word
' host-finalize ' host-finalize map-host-word