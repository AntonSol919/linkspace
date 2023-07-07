# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-participant data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
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

If you want a quick explanation using code check out [Quick Start](https://www.linkspace.dev/code.html).  
[Basics](https://www.linkspace.dev/basics.html) is a high level description.  
The [tutorials](https://www.linkspace.dev/docs/tutorial/index.html) document some practical examples.  
The [Guide](https://www.linkspace.dev/docs/guide/index.html) is a in-depth overview of the API and how it fits together.  

[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try.


Linkspace is not an end-user application. It is a software library(and command line tools) to make more powerful end-users applications.
A GUI frontend to manage groups/domains/keys is outside its scope.

Linkspace is currently in **beta**.

The packet format is stable. Packets created now will be readable in all future versions.

The API is mostly stable but can break now and then.
There are some big features missing:

- No API for deleting.
- The only group exchange process is a bash script.
- LNS only works 'manually'

### Using linkspace

The 3 primary ways of using linkspace are:

- The `lk` CLI [cli/linkspace](./cli/linkspace)
- The Rust library with `linkspace = {git = "https://github.com/AntonSol919/linkspace"}`
- The python [bindings](https://pypi.org/project/linkspace/) with `pip install linkspace`. 

Initial bindings for wasm can be found in [ffi/linkspace-js](./ffi/linkspace-js).
For building from source see [Guide#setup](https://www.linkspace.dev/docs/guide/index.html#setup) or the various README.md's.

You can try out some examples by cloning / downloading and using:

```bash
./join-testexchange
```

Run an application like linkmail to say hi:

```bash
source ./activate
linkmail.py
```

## Development

For a list of todo's and other notes see the dev folder. 

Feel free to send me feedback or ideas!

You open an issue or can contact me directly at <AntonSol919@gmail.com>.

