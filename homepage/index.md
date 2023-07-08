
:::{.definition}
Supernet  [ˈsü-pərˌnet]<br>
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.<br>

e.g. git, bitcoin, linkspace
:::

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to control and administrate (part of) a digital system.
In multi-party systems: there is no total ordering of events, and messages should stay authenticated when passed along.
For this, a supernet is a better tool then our current systems.

Furthermore, supernets can be an alternative to the status quo in which users communicate through a host that has final administrative say over the user's experience.

Linkspace is a supernet with the following highlights:

- Small API
- Fast packet processing (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressing
- Group/Domain split


Linkspace is not an end-user application. 
It is a packet format, and a software library (and command line tools) to build more powerful and useful applications.

If you're comfortable reading bash then [Quick Start](./code.html) is a good place to start.   
If you're not, or prefer a more high level description, begin at the [why](why.html) page. 
The [tutorials](https://www.linkspace.dev/docs/tutorial/index.html) has some practical examples.  
The [Guide](https://www.linkspace.dev/docs/guide/index.html) goes into more depth on the API and technical design.

[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or build from source by cloning the [git](https://github.com/AntonSol919/linkspace) repo. 
