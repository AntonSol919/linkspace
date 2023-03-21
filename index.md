```definition
Supernet  [ˈsü-pərˌnet]
A self-referential multi-user data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.
A communication protocol where the method of exchange is an extraneous concern.
e.g. git, bitcoin, nostr, linkspace
```

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

Linkspace is supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable packets.
- Group/Domain split

To write applications for a supernet requires a different perspective compared to managing sockets.
The challenge is defining a super structure that can work across time and space with noisy incomplete data.
Or treat it as a shared folder. That works as well.

In return a supernet provides a lot of useful properties including:
Serverless, free, extensible, reusable, adaptable, scalable, lockin-resistant, lockout-resistant, verifyable, optional accountability, inherent privacy,fault tolerant etc.

Checkout [Basics](./basics.html) for a simple introduction.
Checkout the [Guide](./docs/guide/index.html) if you're familiar with git and web-servers.

The packet format and index is stable, but the API and various conventions are still in the early stages.
Expect some unimplemented feature, half-baked ideas, rough edges, and the occasional bug.
