# RFC - Up for debate
- Add query seperator for building multiple? ( multi hash get )
- Membership convention. How does a domain app get the members of a group? (probably requires admin key)
- Semantics around removing packets form the local index
- Add predicate type that fails if the predicate is already constrained.
- Error packets. (Users must be able to 'fill' a hash entry with a error packet indicating they do not want it.)
- Integrate/Standardize bloom options for queries
- Add aliases for predicates such that decimal can be used - translate "log_entry<0"  into "i_log:<:[u32:0]"
- Create one standard HTML structure / HTTP URL Request structure.

# Pending

- each instance needs a administrator key. This prevents other programs from messing with LNS names or Group membership. 
- LNS resolver
- Create folder import/export example (Update cli/impex)
- Exchange with bloom filter(+count)
- status-pull should take an optional max_age - allows checking if any process ever set a status without setting the timeout to max.


# API updates

- pull default watch_id == domain/group. lk_watch error on missing :watch
- Probably should error on lk_watch without :watch
- filter / adota style setup for "Set of accepted queries"
- lk_query_print(Some(&str)) -> "group/domain" to print just "group" predicates
- Add custom ABE callback for a user defined scope
- Add FilterScope 'scope' that errors on seing a specific func/eval to, for example, prevent readhash and conf

# Should have

- permit multiple instances to be open
- Packed queries: They're transmitted as text, but they can be packed into bytes.
- :end:HASH option to break on pull request
- :start:HASH option
- :follow:TAG/HASH predicates
- inmem store - an old version exists but we need a proper interface over key-value db
- Wasm
- C API  (vtable NetPkt & CPktHandler)
- predicate-aliases impls (--links)

- tree branch iterator options. currently only create stamp order is supported. Should have: 
-- depth order
-- path asc-desca
-- key order ( might have to be random, having keys be "first" sets a bad incentive)

LNS pubkeys should have the option to favor one name.

- abe : pkt bytes input scope
- abe : SelectLink interface

# Internals

- Detangle field_ids abe and ruletype
- The IPC bus is cross-platform, but maybe slow. Platform specific signals might be better.
- make testset its own crate ( required for selectlink interface )
- alternative naming for abe::expr::list, ablist, abtxt
- review all AS casts
- core::env cleanup
- DGPDExpr should impl ABEValidator and be split up into two types . One where the spath length is know and one where it can be dynamic
- PktPredicates.index(RuleType) -> &mut dyn FieldPred
- :mode:hash-* iteration should use uint_set_info
- Normalize lingo around abe "seperators" and "ctr characters" -- have to pick one
- stack spath/ipath - max size is 250bytes. Could impl copy
- abe - change macro '{}' into '[]'
