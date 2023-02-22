Not everybody wants to host everything all the time.
A potential method for constraining exchange's would be for people to set something like:
(These could be protected by signing them as the local root)

## A list of accepted pull ( DOMAIN:GROUP:/\fexchange/access/pull )
pull-list: #pull list ( must be able to access this list)
push-list: #push list ( must be able to access the push list)
thing0   : ptr to query template
thing1   : ptr to query template
...

## A list of accepted push ( DOMAIN:GROUP:/\fexchange/access/push )

thing0   : ptr to query template
thing1   : ptr to query template
....

## A path where pull requests are set by the domain / read by the exchange
( A DOMAIN:GROUP:/\fexchange/pull )
from : #GET_HASH
++
:start
hash:=:...

## Pull-List
group:=:{group}
domain:=:{domain}
path:=:/\f/queries
:follow
:walk/tag:=:++++++++++++++++


## Push-list
group:=:{group}
domain:=:{domain}
path:=:/\f/queries
:follow

# make exchange publish it
lk status exchange {#:pub} domain DOMAIN
Should return:
[ get : # A list of accepted queries ]
[ set : # A list of accepted push ]


