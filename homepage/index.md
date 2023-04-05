```definition
Supernet  [ˈsü-pərˌnet]
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.
A communication protocol where the method of exchange is an extraneous concern.
e.g. git, bitcoin, nostr, linkspace
```

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to own and administrate a (part of a) digital system.
This is oppose to the current technologies that have users contact a single host,
which acts as the de facto administrator by virtue of hosting the data.

Linkspace is supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable packets.
- Group/Domain split

To write applications for a supernet requires a different perspective compared to managing sockets.
The challenge is defining a super structure that can work across time and space with noisy incomplete data.

In return a supernet provides a lot of useful properties including:
Serverless, free, extensible, reusable, adaptable, scalable, lockin-resistant, lockout-resistant, verifyable, optional accountability, inherent privacy,fault tolerant, etc.

Checkout [Basics](./basics.html) for a simple introduction.
[Download](./download.html) to give it a try and say hi on the test group.
Checkout the [Guide](./docs/guide/index.html) if you're up for some programming.

The packet format and index is stable, but expect some unimplemented feature and rough edges.
