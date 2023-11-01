# v0.5.1

- Python Pkt.data field uses buffer protocol to be zero copy.
- Print query in most-recent-first order - clarify that 'push/parse' them to the front to overwrite previous val
- improve tracing events for database actions
- lk_get_all negative return is used if 'break', otherwise positive 
- change lk_get_hash into lk_get_hashes 

# v0.5.0

- Rename abe context to scope in public api
- add gitea ci workflow
- Make spacename decoding forgiving of superfluous '/'

# v0.4.0

- Rename 'pathname' to 'spacename'
- Make runtime a feature flag - allows wasm compilation of rust library
- ABE: Add option to read UTF8 as plain bytes.
- CLI: Allow UTF8 for templates

# v0.3.1

### General 

- LNS: use tags to store 'until'
- LNS: by-tag uses tag.ends_with instead of eq
- ABE: change ABList from (val,delim?) to (delim?,val)
- ABE: rename 's' (stamp) function to 'us' (microseconds)
- ABE: rename '*=' to 'link:name' (gets the first link with a tag ending with name)

### CLI 

- watch-tree no longer defaults to '**' depth
- cli query option renamed --db-only and --new-only for clarity
- make eval and pktf use a forgiving abe parser. Top level inputs are read as is allowing for unicode and newlines


# v0.3.0
### General

- Change point layout to always include padding up to u64 alignment
- disable mmap by default
- include build_info in output
- pkt-quick and html-quick fmt options
- default domain/groups (set by LK_DOMAIN/LK_GROUP env)
- rename lk_enckey/lk_keyopen => lk_key_encrypt/lk_key_decrypt
- lk_get_all and lk_watch have similar signatures
- split read into read and read_unchecked
- expose the blake3 function
- lk_query(&Q) instead off using lk_query(Some(..))
- reorder arguments to lk_*point
- make all packet length functions explicitly u16
- ABE : rename [env:] to [file:]

### Rust

Remove 'Ptr' type alias

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
