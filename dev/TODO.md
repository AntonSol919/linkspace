# RFC - Up for debate

- API/semantics for removing packets form the local index
- API/semantics for error packets - allow databases 'fill' a entry with a error packet indicating they do not want it.
- Add aliases for predicates such that decimal can be used - translate "log_entry<0"  into "i_log:<:[u32:0]"
- have lk_pull check for exchange status.
- Define query separator -  pack multiple queries back to back
- lk_scan_manual( table, order, start, cb :&dyn NetPkt -> ) where NetPkt stubs to do lookup off values when requested.
- Standardize bloom options for queries
- Standardize notation for "acceptable queries".
- Add option to copy netheader Stamp to database Recv. 
- Membership convention. How does a domain app get the members of a group? (probably requires admin key)
- lk_process recurses if a cb writes a new packets. Maybe add a lk_process_norecurse?

# TODO

## API 
- Add predicate operation type that fails if the predicate is already constrained.
- lk_read should use u32 flags options
- have lk_get_all accept :follow options
- status-watch should take an optional max_age - allows checking if any process has ever set a status without setting the timeout to max.
- lk_pull_close  or lk_stop(pull.qid) by having lk_pull install on_close to overwrite existing req
- permit multiple instances to be open

### Query 
- query 'state's (i_branch) should be an :tree:branch option.
- predicate-aliases impls (--links)
- tree branch iterator options. currently only create stamp order is supported.  
Every segment of a tree key could be in a different order. Most important is the public key.
Having the keys ordered by lexical 'first' might set a bad incentive.
- Its possible to use a byte repr of queries instead of strings - less parsing for exchange processes.
- :hash-start:HASH & :hash-end:HASH option - would expand how ex procs could synchronize - e.g. :mode:log-asc :hash-start:LAST_KNOWN_HASH ie pagenation token

### ABE
- Add a syntax to :follow a subset of links. (useful lk_pull and lns in general)
- Add custom ABE callback for a user defined scope
- '#ab' length-delimited binary format for abtext
- [/links:] macro could include a scope to access the packet. (probabbly want to overwrite links macro in runtime ctx)
- [walk:tagex:tag:tag3:pi]


## LNS 
- LNS : reinit roots
- each instance needs a administrator key. This can prevents other programs from messing with LNS names or Group membership. 
- pubkey should have the option to favor one name.
- UDP resolver
- a 'named by' setup to resolve alice->bob->named_by_bob

## linkspace-cli

- detangle --pkt_in & --read_private options.
- --data-prefixed-size - read first line to determine datablock size, can be combined with with rolling checksum tool that cuts data on a pattern - increasing the chance of producing the same packet from similar data(like rsync)
- get-links - add filters in recursive mode
- [maybe] Split off CLI point functions into standalone tool

## linkspace-py
- Make all 'lk:Linkspace' arguments optional and default to the last session
- Improve function documentation

## linkspace-js
- More WASM functions exposed.
- non-lmdb environment - echodb/indxdb

## linkspace-c
- init 

## Misc
- could have a db env using a single file where tables are created on open.
- Add [pkt-dot] output format
- Create full folder import/export example (Update cli/impex)
- Exchange with bloom filter(+count)
- Port anyhost exchange to rust 

## Internals

- `lk` help strings should not evaluate on every run
- spacename (const) macro's need a rewrite.
- lk_pull should register a on_close to overwrite the packet signaled to the exchange.
- Detangle field_ids abe and ruletype
- The IPC bus is cross-platform, but maybe slow. Platform specific signals might be better.
- make testset its own crate ( required for selectlink interface )
- core::env split off into its own crate? Needs support for alternative storage 
- common:rx needs a rewrite. Lots of cruft from a time it was a multithreaded dispatch.  
Probably want a non-borrow-lock solution.
instead of a cmd queue we could do a 'close' as
WatchEntry{ update_now: RefCell<Result<(),Option<Box<WatchEntry>>>>> ...} and check update_now after pkt_handle is complete
- clarify nested query open semantics.

- DGPDExpr should impl ABEValidator and be split up into two types. One where the spacename length is know and one where it can be dynamic
- PktPredicates.index(RuleType) -> &mut dyn FieldPred
- :mode:hash-* iteration should use uint_set_info
- Stack spacename - max size is 250bytes. Could impl copy
- abe! macro - change macro '{}' into '[]'
- Making abe 'list_functions' instead be a visitor pattern with &mut cb(Func) could simplify some things. 
- blake3 allocates >1700 bytes to hash <= 2^64 bytes - when we could do with a lot less


