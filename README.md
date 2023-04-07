# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-participant data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is an extraneous concern.
> e.g. git, bitcoin, nostr, linkspace


In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to own and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host,
which acts as de facto administrator by virtue of hosting the data.

[Linkspace](https://antonsol919.github.io/linkspace/index.html) is supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable data
- Group/Domain split

Check out [Basics](https://antonsol919.github.io/linkspace/index.html#basics) for an introduction.
[Download](https://antonsol919.github.io/linkspace/index.html#download) to give it a try and say hi on the test group.
Check out the [Guide](./docs/guide/index.html) if you're up for some programming.

The packet format and index are stable, but expect some unimplemented features and rough edges.

# Guide

If you're interested in development, it is suggested to start at the [guide](https://antonsol919.github.io/linkspace/docs/guide/index.html)

Or jump straight to a section:

- [Point](https://antonsol919.github.io/linkspace/docs/guide/index.html#Point) creation
- [ABE](https://antonsol919.github.io/linkspace/docs/guide/index.html#ABE) - ascii byte expressions - a language for manipulating and templating bytes
- [Query](https://antonsol919.github.io/linkspace/docs/guide/index.html#Query) - Addressing and filtering of packets with predicates and options
- [Linkspace](https://antonsol919.github.io/linkspace/docs/guide/index.html#Linkspace) instance - Locally indexed packets and new packet processing functions
- [Conventions](https://antonsol919.github.io/linkspace/docs/guide/index.html#Conventions)

# Building

The 3 primary ways of using linkspace are:
The `lk` CLI [cli/linkspace](./cli/linkspace), the Rust library [crates/liblinkspace](./crates/liblinkspace), and the python library [ffi/liblinkspace-py](./ffi/liblinkspace-py)

## Development

Checkout the ./dev folder for missing features and whats currently on my TODO.
A great way to help out would be to improve or build a new ./examples/linkmail-py.

### About

Linkspace is currently an unfunded project
Meaning I still do other stuff for food, and won't be available all the time.

Please contact me directly if you're interested in supporting the project.
Or if you want to talk about specific applications.

### Contact

Of course the preferred way is to try and contact me on the test server.
For the less adventurous you can use <antonsol919@gmail.com>.
