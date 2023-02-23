# The Linkspace Protocol - a general purpose supernet

> Supernet  [ ˈsü-pərˌnet ]
> A self-referential multi-user data organization protocol whose primary 
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is a extraneous concern. 
> e.g. git, bitcoin, nostr, the linkspace protocol 

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

[The Linkspace Protocol](https://antonsol919.github.io/linkspace/index.html)  is supernet with the following highlights:
- Small and powerfull API
- Fast ( Blake3, no json/b64 encoding, well aligned fields )
- Path ( url like ) addressable packets.
  - group/domain split

To write applications for a supernet requires a different perspective compared to managing sockets.
The challenge is defining a super structure that can work across time and space with noisy incomplete data.

In return a supernet provides a lot of usefull properties including:
Serverless, lockin-resistant, lockout-resistant, extensible, scalable, accountable, privacy, replaceability, verifyable, (offline)availble, etc.

If you don't know those words checkout [ELI5](https://antonsol919.github.io/linkspace/eli5.html) for a more gentle introduction to the general idea.

The packet format and index is stable, but the API and various conventions are still in the early stages.
Expect unimplemented feature, half baked ideas , rought edges, and the occasional bug.
See the ./dev folder for more.

# Usage

There are currently 3 ways using linkspace:
The `lk` CLI [cli/linkspace](./cli/linkspace), the Rust library [crates/liblinkspace](./crates/liblinkspace), and the python library [ffi/liblinkspace-py](./ffi/liblinkspace-py)
A linkspace instance is a directory containing a database and auxiliary data.
Multiple applications use it at the same time.

Its suggested to start at the [guide](https://antonsol919.github.io/linkspace/docs/guide/index.html) ( or build it locally with `make docs`)

Or jump straight to a section:
- [Point](https://antonsol919.github.io/linkspace/docs/guide/index.html#point) creation
- [ABE](https://antonsol919.github.io/linkspace/docs/guide/index.html#abe) - ascii byte expressions - a language for manipulating and templating bytes
- [Query](https://antonsol919.github.io/linkspace/docs/guide/index.html#query) - Addressing and filtering of packets with predicates and options
- [Linkspace](https://antonsol919.github.io/linkspace/docs/guide/index.html#linkspace) instance - Locally indexed packets and new packet processing functions
- [Conventions](https://antonsol919.github.io/linkspace/docs/guide/index.html#conventions) instance - Locally indexed packets and new packet processing functions
