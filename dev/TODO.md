
# RFC - Up for debate

- Membership convention. How does a domain app get the members of a group? (probably requires admin key)
- Semantics around removing packets form the local index
- Error packets. (Users must be able to 'fill' a hash entry with a error packet indicating they do not want it.)
- Integrate/Standardize bloom options for queries
- Add aliases for predicates such that decimal can be used - translate "log_entry<0"  into "i_log:<:[u32:0]"
- Specify standardized HTML structure / HTTP URL Request structure.
- i_branch/i_db/i predicate names 
- have lk_pull check for exchange status. 
- Add query seperator for building multiple? ( multi hash get )


# Pending

- each instance needs a administrator key. This prevents other programs from messing with LNS names or Group membership. 
- LNS UDP resolver
- LNS recursive :local
- Port anyhost exchange to rust 
- Create folder import/export example (Update cli/impex)
- Exchange with bloom filter(+count)
- status-pull should take an optional max_age - allows checking if any process ever set a status without setting the timeout to max.
- lk_pull_close 
- LNS : Drop some roots, remove root names


# API updates

-

# Should have

- permit multiple instances to be open
- Packed queries: They're transmitted as text, but they can be packed into bytes.
- :end:HASH option.
- :start:HASH option
- Add predicate operation type that fails if the predicate is already constrained.
- notation for "acceptable queries" (see adota for some ideas).
- inmem store - an old version exists but we need a proper interface over key-value db
- Compile to WASM
- C API  (vtable NetPkt & CPktHandler)
- tree branch iterator options. currently only create stamp order is supported.  
Every segment of a tree key could be in a different order. Most important is the public key.
Having the keys ordered by 'first' might set a bad incentive. 
- LNS pubkey should have the option to favor one name.
- Add custom ABE callback for a user defined scope
- abe : Missing a syntax to select a subset of links. (useful for :follow and lns in general)
- predicate-aliases impls (--links)

# Internals

- lk_pull should register a on_close to overwrite the packet signaled to the exchange.
- Detangle field_ids abe and ruletype
- should prob switch lmdb to libmdbx
- The IPC bus is cross-platform, but maybe slow. Platform specific signals might be better.
- make testset its own crate ( required for selectlink interface )
- review all AS casts
- core::env cleanup
- common:rx needs a rewrite. Lots of cruft from a time it was a multithreaded dispatch.  
Probably want a non-borrow-lock solution.
instead of a cmd queue we could do a 'close' as
WatchEntry{ update_now: RefCell<Result<(),Option<Box<WatchEntry>>>>> ...} and check update_now after pkt_handle is complete
must clarify nested query open semantics.
- DGPDExpr should impl ABEValidator and be split up into two types. One where the spath length is know and one where it can be dynamic
- PktPredicates.index(RuleType) -> &mut dyn FieldPred
- :mode:hash-* iteration should use uint_set_info
- Normalize lingo around abe "seperators" and "ctr characters"
- stack spath/ipath - max size is 250bytes. Could impl copy
- abe - change macro '{}' into '[]'

# linkspace-cli

detangle --pkt_in & --read_private options.


# linkspace-py
Improve function documentation
