:::{.definition}
Supernet  [ˈsü-pərˌnet]<br>
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.<br>
A communication protocol where the method of exchange is an extraneous concern.<br>

e.g. git, bitcoin, linkspace
:::

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

[Basics](./basics.html) gives a high level introduction.  
The [tutorials](./docs/tutorial/index.html) has some practical examples.  
The [Guide](./docs/guide/index.html) is an in-depth overview of the API and how it fits together.  

[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or build from source by cloning the [git](https://github.com/AntonSol919/linkspace) repo. 
