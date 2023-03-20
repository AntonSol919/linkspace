There are a couple interesting usecases for HTTP bridges. 

## Direct HTML 
By mapping URLs to paths you can create a webpage that uses hyperlinks to browse around.

## wasm
A wasm runtime runtime can either be a full fledge DB, or can access 
over a web page. 

## (ab)using cashing 
While linkspace and LNS are still young, HTTP cashing is simple to use.



# Notes on API Some ideas on HTTP bridges
A publicly facing server exposing a HTTP bridge should
at least impl the following API at some root

## API
Any ?hash should forward to its matching /t/ or /hash

- GET ./t/[#:pub]/DOMAIN[/COMP]* + ?create>  // latest item. where COMP bytes can be encoded as :00 upto :FF  . '?mode=txt|json|html'

- GET ./hash/HASH ?mode=txt|json|html
redirects to ./t/ ?create= ?hash= 
