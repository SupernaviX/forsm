: test
  26 0 do i 97 + emit loop
;

cr test

: test
  [char] ( emit test [char] ) emit
;

: test-string
  cr s" Hello world!" type
;

: test-print-string
  cr ." Kalloo kallay o frabjous day!"
;

test-string
test-print-string

cr test

cr parse-name flibbertigibbet type

cr 123 0 ud.
cr -123 abs s>d ud.
cr -123 s>d d.
cr 123 u.
cr 123 .
cr -123 .

cr 1 2 3 .s
cr .
cr .
cr .s
cr .
cr .s