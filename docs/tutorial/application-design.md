Some notes on application level algorithm design. i.e. how to build something with linkspace. 

# Associative state transition functions

'State' in this context refers to any mutating variable.

The challenge of building a program for a distributed log is creating an algorithm
that, regardless of the _order_ of events received, the state is equal to any other order.

I.e. A+(B+C) == (A+B)+C.

or in a more practical and common syntax .

Si([e1,e2,e3]) == Si([e2,e3,e1]) == Si([e3,e1,e2]) == ... 

Where Si is a function processing a list of events. 

Note that this property is transitive.
i.e. if this is true for the result of Si then it is true for F(Si([..])) and G(F(Si([..]))) given that F and G are pure functions.

In other words, it can be built by using intermediary states built with such a function.

For example (and a useful starting point) is the linkspace database.
The hash index and the tree index both have this property.

Inserting into the hash index is by definition a associative state transition function[^1]. 

[^1]: while the hashes don't collide - which is taken for granted with 2^256 bits. 

The tree is index ordered by the tuple: 
 ( group, domain, space_depth, spacename, pubkey, create stamp, hash )
 
This is also obviously equal regardless of insertion order. 

Another intermediary state is a 'reverse-link' table, i.e. a Map<Hash,Vec<Pkt>>, to find all Pkts with a link to Hash. (This is not included in the database because the overhead was chosen to be too expensive.)

Practically this means you can choose to create a function that reads the database and creates a state.
Then watch for packets that _could_ update the final state and rerun the function.
This is not always the fastest but its simple, correct for any client, and fast enough for most use cases.

Its still up to the application to add additional guarantees about authenticity and order across the network. 
A peer does not automatically know its missing any packets. One solution is to have a single peer create a (signed) summary every now and then with a list of hashes the packets they acknowledge. The CAP theorem limits will always limit a distributed system to a 'best effort' solution. 

Note that in this 'single summarizing peer' setup the effect is similar to how the current web works. 
A central host administrates what is part of their platform and what is not.

# Event loops 
For most applications you want a strict split between gui/interface state and the organization/process state encoded in packets.

I've found a good approach is to:

- have a single function process packets to update a the process' state.
- Limit the gui to reading the process' state. Any modifications should be made by creating packets.

You can use different threads, but a simple single-thread alternative is to have the gui thread call lk_process.


