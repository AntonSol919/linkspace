# RFC - Up for debate
- Add query seperator for building multiple? ( multi hash get )
- Group Membership
- Semantics around removing packets form the local index
- Add predicate type that fails if the field already contains another constraintpredicate
- Error packets. (Users must be able to 'fill' a hash entry with a error packet indicating they do not want it.)
- Integrate/Standardize bloom options for queries?

# Pending API updates

pull default watch_id == domain/group. lk_watch error on missing :watch
Probably should error on lk_watch without :watch

misc::lk_inspect_watches
Incongruity ABE '?' {/?..} returns ABE str, {u16:2/?u} returns val -> probably want "b:..." , u8:12, #:me

[?] lk_query_print(Some(&str)) -> "group/domain" to print just "group" predicates

- Add custom ABE callback for a user defined scope ( opt add fs_scope )
- Add FilterScope 'scope' that errors on seing a specific func/eval to, for example, prevent readhash and conf
- [abe] add quote eval {/q:a{bc}} => the byte seq "a:{bc}"

# Improvements - Things pending impl

- Create folder import/export example (Update cli/impex)
- Bloom filter(+count) query option
- Enable {@:me:local} lns name resolution (impl in LocalNS)
- :follow:TAG/HASH predicates
- inmem store
- wasm
- C API  (vtable NetPkt & CPktHandler)
- store query in watch entry as string
- :end:HASH option to break on pull request
- :start:HASH option
- status_pull should probably take an optional max_age, this allows checking if any process ever set a status without setting the timeout to max.
- allow multiple instances to be open


- Packed queries: They're transmitted as text, but they can be packed into bytes.

- [internal] core::matcher cleanup
- [internal] core::env cleanup
- [internal] DGPDExpr should impl ABEValidator and be split up into two types . One where the spath length is know and one where it can be dynamic
- [internal] PktPredicates.index(RuleType) -> &mut dyn FieldPred
- [internal] :mode:hash-* iteration should use set_info
- [internal] Normalize lingo around abe "seperators" and "ctr characters" -- have to pick one
- [internal] Retype ApplyResult as EvalResult = ControlFlow<Break=Result<Vec<u8>,Error>>; 


# Issues - Things considered broken

- review all AS casts
- Review query expression resolution during pull.
- Limit anyhost requests
- LNS resolver
- add 'read-only (root)' key file for important information (e.g. active domain,group list ) + create a scope to {root}
tryout lns as stages, each stage does not require to be in path . 
{#:hello:world:this} == {&#:{#:hello:world}:this} == {&#:{&#:{#:hello}:world}:this}

- pktpredicates add comp[0-8]
- predicate-aliases impls
 --links

- tree branch iterator options.
-- path iter order. on pathcomp, and on pubkey ( lower pubkeys always come first currently )
