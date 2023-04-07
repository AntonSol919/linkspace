```definition
Supernet  [ˈsü-pərˌnet]
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.
A communication protocol where the method of exchange is an extraneous concern.
e.g. git, bitcoin, nostr, linkspace
```

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to own and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host,
which acts as de facto administrator by virtue of hosting the data.

Linkspace is a supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable data
- Group/Domain split

Check out [Basics](./basics.html) for an introduction.
[Download](./download.html) to give it a try and say hi on the test group.
Check out the [Guide](./docs/guide/index.html) if you're up for some programming.

The packet format and index are stable, but expect some unimplemented features and rough edges.
