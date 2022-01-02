\ Use a virtual IP and return stack to emulate words during compilation
variable v-ip
create v-rstack 256 allot
here constant v-r0
variable v-rp
v-r0 v-rp !
: v->r ( value -- ) -4 v-rp +! v-rp @ ! ;
: v-r> ( -- value ) v-rp @ @ 4 v-rp +! ;
: v-rdepth ( -- u ) v-r0 v-rp @ - 2/ 2/ ;

: callable' ( -- callable )
  ['] lit , v-' v-@ ,
; immediate

\ Given an XT from the virtual interpreter, run it
: v-execute' ( v-xt -- )
  >r r@ v-@
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
    callable' cset of v-cset endof
    callable' creset of v-creset endof
    callable' >r of v->r endof
    callable' r> of v-r> endof
    callable' dup of dup endof
    callable' drop of drop endof
    callable' 2drop of 2drop endof
    callable' swap of swap endof
    callable' =0 of =0 endof
    callable' <>0 of <>0 endof
    callable' + of + endof
    callable' 1+ of 1+ endof
    callable' 1- of 1- endof
    callable' * of * endof
    callable' and of and endof
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

\ Support emulation of words provided by the host
create host-words 16 2* cells allot
variable host-words#
0 host-words# !
: map-host-word ( from-xt to-xt -- )
  host-words# @ 2* cells host-words + tuck ( from-xt addr to-xt addr )
  cell + ! !
  1 host-words# +!
;
: host-execute ( xt -- ran? )
  host-words# @ 0 ?do
    i 2* cells host-words + ( xt mapping )
    2dup @ = if
      nip
      cell + @ execute
      true unloop exit
    else drop
    then
  loop
  false
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

: v-source ( -- c-addr u )
  [v-'] tib v-execute v-@
  [v-'] tib# v-execute v-@
;
: v-refill ( -- ? )
  0 [v-'] >in v-execute v-! \ reset >IN
  TIB_BASE dict[] TIB_CAPACITY v-source-fid @ ( c-addr u1 fid )
  read-line throw ( u2 more? )
  swap [v-'] tib# v-execute v-! \ write how much we read
;

: v-parse-area ( -- vc-addr u ) v-source [v-'] >in v-execute v-@ /string ;
: v-parse-consume ( u -- ) [v-'] >in v-execute v-+! ;
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
: vstr>str ( vc-addr u -- c-addr u ) swap dict[] swap ;

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

' . ' . map-host-word
' cr ' cr map-host-word
' type ' v-type map-host-word
' .s ' .s map-host-word

\ compilation functions
: v-\ ( -- ) -1 v-parse 2drop ;
: v-( ( -- ) [char] ) v-parse 2drop ;
: v-variable ( -- )
  v-parse-name [v-'] header v-execute
  0 v-,
;
: v-constant ( value -- )
  v-parse-name [v-'] header v-execute
  [v-'] (docon) v-execute [v-'] xt, v-execute
  v-,
;
: v-: ( -- )
  v-parse-name [v-'] header v-execute
  [v-'] (docol) v-execute [v-'] xt, v-execute
  [v-'] hide v-execute
  [v-'] ] v-execute
;

' parse ' v-parse map-host-word
' parse-name ' v-parse-name map-host-word
' \ ' v-\ map-host-word
' ( ' v-( map-host-Word
' variable ' v-variable map-host-word
' constant ' v-constant map-host-word
' : ' v-: map-host-word
