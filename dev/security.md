
# Filesystem access
Anything with file level write access to the database can compromise a lot.
In release builds the packets read from the database are expected to be valid.
An attacker could rewrite the publickey of a packet and break systems reliant on it.

This can only be solved by only exposing the interface.
Such as through wasm.
This is a todo.

# Without an admin key
The local LNS entries are (currently) unsigned.
This means any proc can create a new entry and break assumptions about names.

# Eval access
Most of the time ABE expressions are trusted.
But not always, so by default some are limited or disabled. 

By default expressions :
- can't readhash from [#:0]
- can't read ENV variables. 
- can read files in $LK_DIR/file/..

# Eval - (currently irrelevant)
ABE's "eval" and "encode" for a function/eval 'ID' visits scopes in order and attempt to resolve into a value.
When this fails - it continues walking.
If a process could add a scope, they can add add '#' or other functions.
This is considered fine - an malicious process does not gain anything new from modifying eval/encode.

