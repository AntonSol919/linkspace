# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-participant data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is an extraneous concern.
> e.g. git, bitcoin, linkspace

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to control and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host which acts as the de facto administrator.

[Linkspace](https://www.linkspace.dev/index.html) is supernet with the following highlights:

- Small API
- Fast packets (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressing
- Group/Domain split

[Basics](https://www.linkspace.dev/basics.html) gives a high level introduction.
Check out the [tutorials](https://www.linkspace.dev/docs/tutorial/index.html) to see an example of building an application.
For a technical document regarding the API and CLI see the [Guide](https://www.linkspace.dev/docs/guide/index.html).
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
The [Guide#setup](https://www.linkspace.dev/docs/guide/index.html#setup) has more details.

## Development

Checkout the ./dev folder for missing features and whats currently on the TODO.

Linkspace currently has no financial backing.
Development depends on available free time.
