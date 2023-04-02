# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-participant data organization protocol whose primary
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

Checkout [Basics](https://antonsol919.github.io/linkspace/basics.html) for a simple introduction.
[Download](https://antonsol919.github.io/linkspace/download.html) to give it a try and say hi.
Checkout the [Guide](https://antonsol919.github.io/linkspace/docs/guide/index.html) if you're up for some programming.

The packet format and index is stable, but expect some unimplemented feature and rough edges.

# Guide

It is suggested to start at the [guide](https://antonsol919.github.io/linkspace/docs/guide/index.html)

Or jump straight to a section:

- [Point](https://antonsol919.github.io/linkspace/docs/guide/index.html#Point) creation
- [ABE](https://antonsol919.github.io/linkspace/docs/guide/index.html#ABE) - ascii byte expressions - a language for manipulating and templating bytes
- [Query](https://antonsol919.github.io/linkspace/docs/guide/index.html#Query) - Addressing and filtering of packets with predicates and options
- [Linkspace](https://antonsol919.github.io/linkspace/docs/guide/index.html#Linkspace) instance - Locally indexed packets and new packet processing functions
- [Conventions](https://antonsol919.github.io/linkspace/docs/guide/index.html#Conventions)

# Building

The 3 primary ways of using linkspace are:
The `lk` CLI [cli/linkspace](./cli/linkspace), the Rust library [crates/liblinkspace](./crates/liblinkspace), and the python library [ffi/liblinkspace-py](./ffi/liblinkspace-py)

# Development

See the ./dev folder for the roadmap/missing features.

# Misc

This is currently a privately financed project representing done as a part time job.
Ideally this project could fund one or more fulltime devs through sales of [LNS](https://antonsol919.github.io/linkspace/lns.html) names.