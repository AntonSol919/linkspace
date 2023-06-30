# The problem

The exchange problem is:

Any set of group members in any network topology have to efficiently
exchange the minimum set of packets that satisfy the domain applications request (initiated by the user).

For example, the following usecases require different types of exchange processes to be efficient.

- A group of a few dozen friends sync everything with eachother.
- A public message board. Multiple hosts carry part of the data.
- A company creates a network of dedicated servers for global distribution sharding on the pkt hash.
- A smart meter on the local LAN starts dumping its state as keypoint's on a UDP broadcast channel.

Linkspace doesn't have a blessed way to synchronize so you'll have to define your own.
The ubits[0..2] and stamp fields are specifically meant for groups exchange processes to communicate by the group exchange process.
They're included when saved in the database and predicates can be added to any query.

A system that does a 'full sync' between group members is trivially correct.

If you control every device and you want full sync you can pretty quickly define some topology together using `lk route` and `lk filter/ignore`. 

```
netcat server1 | lk route ubits0:=:[u32:$SERVER1_ID] --write db
```

and by saving some stamp for the last message sent.

```
lk watch-log --bare -- "recv:>:$LAST_MSG" | lk ignore ubits0:=:[u32:$SERVER1_ID]  | netcat server1`
```


The anyhost example is a basic server/client model that does something like this - as well as follow the basics for pull requests so clients don't have to fully sync. It currently does not have any access control. 

Here are some ideas/notes: 

Sending to much data is never a problem except for overhead.

The `pull` convention is used for applications to signal their interests. 
This includes dropping their interest by reusing the 'qid'.
(A 'correct' application does not fail if you ignore either of them, but follow them when possible.)

Queries are designed for concatenation to make sense.
i.e. adding "group:=:[#:ourgroup]" to a query gives you the empty set if the user is trying to access group:=:[#:yourothergroup].
There are some initial designs to further expand this ability.
(Somewhat related is the design notes in exchange-limit.md)

You can freely add additional options such as ":myoption:myvalue/thing..." to a query.

The recv predicate depends on the context. 
Reading from the database the recv is set to the time the packet was received:  `lk watch-log --bare -- "recv:>:[now:-1D]"`
But when reading from a pipe it is set to when the pipe reads the packet: `lk watch-log | lk filter "recv:>:[now:-1D]"`
In both cases the predicate `recv:<:[now:+1m]` would stop the process after 1 minute.

By default the hop field is automatically incremented - the cli have flags/env vars to change this. ( this functionality is not yet exposed in the bindings but you can directly mutate the hop field)


The :follow query option is preferred compared to hash:=:_. 
However, this could follow links pointing outside the group.
There are two options to deal with this: 

- add a final filter before transmitting
- accept that leaking a hash is the same as leaking the data. 

I personally think the latter is right - exposing the hash but expecting its content to stay secret is usually not what you want, and in the case that it is you shouldn't be saving the content in the same database anyways.


A good way to limit the amount synced between two points with no prior knowledge is to add a bloom filter + packet count to a request.

# Missing pieces: 

- We're missing a convention on 'group membership'.
- There is no consensus wether :follow is implied or not by lk_pull
