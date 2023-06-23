# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-participant data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is an extraneous concern.
> e.g. git, bitcoin, linkspace


In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to control and administrate (part of) a digital system.
In multi-party systems: there is no total ordering of events, and messages should stay authenticated when passed along.
For this, supernets provide a better abstraction then building on top of streams of data such as TCP/IP.

Furthermore, supernets can be an alternative to the status quo in which users communicate through a host that is given total administrative rights over their experience.

Linkspace is a supernet with the following highlights:

- Small API
- Fast packet processing (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressing
- Group/Domain split

[Basics](https://www.linkspace.dev/basics.html) gives a high level introduction.
The [tutorials](https://www.linkspace.dev/docs/tutorial/index.html) has some practical examples. 
The [Guide](https://www.linkspace.dev/docs/guide/index.html) is a in-depth overview of the API and how it fits together. 

[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try.

Linkspace is currently in **beta**.

That means the packet format is stable. Points created now will be readable in all future versions.

The API is mostly stable but will break now and then.
There are some features missing:

- There is no API for deleting yet.
- The only group exchange program is a bash script.
- LNS only works 'manually'
- Other todo's found in the /dev folder

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

Checkout the ./dev folder for missing features and what's currently on the TODO.

Linkspace currently has no financial backing.
Development depends on available free time.
