# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-user data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is an extraneous concern.
> e.g. git, bitcoin, nostr, linkspace

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

[Linkspace](https://antonsol919.github.io/linkspace/index.html)  is supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable packets.
- Group/Domain split

To write applications for a supernet requires a different perspective compared to managing sockets.
The challenge is defining a super structure that can work across time and space with noisy incomplete data.
Or treat it as a shared folder. That works as well.

In return a supernet provides a lot of useful properties including:
Serverless, free, extensible, reusable, adaptable, scalable, lockin-resistant, lockout-resistant, verifyable, optional accountability, inherent privacy,fault tolerant etc.

Begin with the [Basics](https://antonsol919.github.io/linkspace/docs/guide/basics.html),
then checkout the [Guide](https://antonsol919.github.io/linkspace/docs/guide/index.html).

The packet format and index is stable, but the API and various conventions are still in the early stages.
Expect some unimplemented feature, half-baked ideas, rough edges, and the occasional bug.

See the ./dev folder for more.

# Usage

There are currently 3 ways using linkspace:
The `lk` CLI [cli/linkspace](./cli/linkspace), the Rust library [crates/liblinkspace](./crates/liblinkspace), and the python library [ffi/liblinkspace-py](./ffi/liblinkspace-py)
A linkspace instance is a directory containing a database and auxiliary data.
Multiple applications use it at the same time.

It is suggested to start at the [guide](https://antonsol919.github.io/linkspace/docs/guide/index.html) (or build it locally with `make docs`)

Or jump straight to a section:

- [Point](https://antonsol919.github.io/linkspace/docs/guide/index.html#Point) creation
- [ABE](https://antonsol919.github.io/linkspace/docs/guide/index.html#ABE) - ascii byte expressions - a language for manipulating and templating bytes
- [Query](https://antonsol919.github.io/linkspace/docs/guide/index.html#Query) - Addressing and filtering of packets with predicates and options
- [Linkspace](https://antonsol919.github.io/linkspace/docs/guide/index.html#Linkspace) instance - Locally indexed packets and new packet processing functions
- [Conventions](https://antonsol919.github.io/linkspace/docs/guide/index.html#Conventions)

