# v0.3.0
### General
- rename lk_enckey/lk_keyopen => lk_key_encrypt/lk_key_decrypt
- lk_get_all and lk_watch have similar signatures
- split read into read and read_unchecked
- expose the blake3 function
- lk_query(&Q) instead off using lk_query(Some(..))
- reorder arguments to lk_*point
- make all packet length functions explicitly u16
- ABE : rename [env:] to [file:]

### Python

### CLI
- rename printf to pktf
- new '--data*' options.
- LK_CHECK_CACHE => LK_SKIP_HASH
- get-links --recurs

### js/wasm
- create first bindings (lk_read/lk_write , lk_*point)

# v0.2.0
### General
- add ./activate to both the repo and zip for a quick start
- Simplify lk_process_while
- Use env variables: root => LK_DIR, LK_KEY => LK_KEYNAME, LK_PASS
- rename query :id to :qid
- rename comp0-8 into path0-8
- ABE: add `~utf8` function for lossy printing  & use in default_pkt_fmt
### Python
- Improve docs and type hints
- Switch callback semantics, return true to break/finish.
- Improve errors
- impl __hash__, __eq__ etc for more types
### CLI
- lk: change --link option to be a list in tail position (useful with xargs)
- lk: add --pkt-in cli option instead of assuming stdin
