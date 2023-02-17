# RFC - Up for debate
- Membership convention
- Add query seperator for building multiple? ( multi hash get )
- Group Membership
- AdotO group filter?
- Semantics around removing packets form the local index
- Add predicate type that fails if the field already contains another constraintpredicate
- Typed (bitflags) query_push_predicate + return bitset of updated fields
- Error packets. (Users must be able to 'fill' a hash entry with a error packet indicating they do not want it.)
- Incongruity ABE '?' {/?..} returns ABE str, {u16:2/?u} returns val

# Pending API updates
convention::status_update
convention::status_view

misc::lk_inspect_views

[?] lk_query_print(Some(&str)) -> "group/domain" to print just "group" predicates

[cli] - print-query --as-txt flag  to match lk_query_print

- Add custom ABE callback for a user defined scope ( opt add fs_scope )
- Add scope filters that error on seing a specific func/eval to, for example, prevent readhash and conf
- [abe] add quote eval {/q:a{bc}} => the byte seq "a:{bc}"

# Improvements - Things pending impl
- rename ubits to hbits sbits gbits , dbits
- Enable {@:me:local} lns name resolution

- :follow:TAG/HASH predicates
- Specify/rename ubits for domain use
- inmem store
- wasm
- C API  (vtable NetPkt & CPktHandler)
- store query in watch entry as string / change view signature to query
- /start option
- Proper errors for liblinkspace

- [internal] core::matcher cleanup
- [internal] core::env cleanup
- [internal] PktPredicates.index(RuleType) -> &mut dyn FieldPred
- [internal] :mode:hash iteration should use set_info
- [internal] Normalize lingo around abe "seperators" and "ctr characters" -- have to pick one
- [internal] rename reeval to eval
- [internal] Retype ApplyResult as EvalResult = ControlFlow<Break=Result<Vec<u8>,Error>>; 


# Issues - Things actually broken
- swap lns order for #

- Loading query from packet should only follow prepend_query
- review all AS casts
- LNS resolver
- add 'read-only (root)' key file for important information (e.g. active domain,group list ) + create a scope to {root}
tryout lns as stages, each stage does not require to be in path . 
{#:hello:world:this} == {&#:{#:hello:world}:this} == {&#:{&#:{#:hello}:world}:this}

- pktpredicates add current todo!()
- predicate-aliasses impls
 --links

- branch iterator options.
-- path iter order. on pathcomp, and on pubkey ( lower pubkeys always come first currently )
