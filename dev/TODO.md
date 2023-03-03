# RFC - Up for debate
- Add query seperator for building multiple? ( multi hash get )
- Group Membership
- Semantics around removing packets form the local index
- Add predicate type that fails if the set is already constrained.
- Error packets. (Users must be able to 'fill' a hash entry with a error packet indicating they do not want it.)
- Integrate/Standardize bloom options for queries?
- Add aliases for predicates such that decimal's can be used - translate "log_entry<0"  into "i_log:<:[u32:0]"

# API updates

- pull default watch_id == domain/group. lk_watch error on missing :watch
- Probably should error on lk_watch without :watch
- Incongruity ABE '?' [/?..] returns ABE str, [u16:2/?u] returns val -> probably want "b:..." , u8:12, #:me
- lk_query_print(Some(&str)) -> "group/domain" to print just "group" predicates
- Add custom ABE callback for a user defined scope
- Add FilterScope 'scope' that errors on seing a specific func/eval to, for example, prevent readhash and conf

# Pending

- LNS resolver
- Create folder import/export example (Update cli/impex)
- Exchange with bloom filter(+count)
- status-pull should take an optional max_age - allows checking if any process ever set a status without setting the timeout to max.

# Should have

- permit multiple instances to be open
- add 'read-only (root)' key file for important information (e.g. active domain,group list ) + create a scope to {root}
- Packed queries: They're transmitted as text, but they can be packed into bytes.
- :end:HASH option to break on pull request
- :start:HASH option
- :follow:TAG/HASH predicates
- inmem store - an old version exists but we need a proper interface over key-value db
- Wasm
- C API  (vtable NetPkt & CPktHandler)
- predicate-aliases impls (--links)

- tree branch iterator options.
-- path iter order. on pathcomp, and on pubkey ( lower pubkeys always come first currently )


# Internals
- review all AS casts
- core::matcher cleanup
- core::env cleanup
- DGPDExpr should impl ABEValidator and be split up into two types . One where the spath length is know and one where it can be dynamic
- PktPredicates.index(RuleType) -> &mut dyn FieldPred
- :mode:hash-* iteration should use set_info
- Normalize lingo around abe "seperators" and "ctr characters" -- have to pick one
- Retype ApplyResult as EvalResult = ControlFlow<Break=Result<Vec<u8>,Error>>; 
