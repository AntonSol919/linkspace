# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-participant data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is an extraneous concern.
> e.g. git, bitcoin, nostr, linkspace

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to own and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host which acts as the de facto administrator.

[Linkspace](https://antonsol919.github.io/linkspace/index.html) is supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable data
- Group/Domain split

[Basics](https://antonsol919.github.io/linkspace/index.html#basics) gives a high level introduction of the entire system.
Check out the [Guide](https://antonsol919.github.io/linkspace/docs/guide/index.html) if you're interested in the technical details.
[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try and say hi.

The packet and database layout are stable, but some things are incomplete or undocumented.

Any feedback, questions, and ideas for improvements are welcome!

Of course the preferred way is to send a message to the test group as explained below.
For the less adventurous you can open an issue on GitHub.

# Quick start

Build and start an exchange process to a public test server:

```bash
./connect-test-pub1
```

Run an application like linkmail to say hi:

```bash
source ./activate
linkmail.py
```

# Building

The 3 primary ways of using linkspace are:
The `lk` CLI [cli/linkspace](./cli/linkspace), the Rust library [crates/linkspace](./crates/linkspace), and the python library [ffi/linkspace-py](./ffi/linkspace-py)
The [Guide#setup](https://antonsol919.github.io/linkspace/docs/guide/index.html#setup) has more details.

With the scripts in ./emulate you can set up and connect local instances.

## Development

Checkout the ./dev folder for missing features and whats currently on the TODO.

Linkspace is currently an unfunded project.
Meaning I do other stuff to make a living and won't be available all the time.
