<div class="definition">
Supernet  [ˈsü-pərˌnet]<br>
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.<br>
A communication protocol where the method of exchange is an extraneous concern.<br>

e.g. git, bitcoin, linkspace
</div>

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
Check out the [tutorials](./docs/tutorial/index.html) to see an example of building an application.
For a technical document regarding the API and CLI see the [Guide](./docs/guide/index.html).

[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try and say hi.

The packet and database layout are stable, but some things are still in active development so expect the occasional breaking change.

Any feedback, questions, and ideas for improvements are welcome!

Of course the preferred way is to try and send a message to the test group.
For the less adventurous you can open an issue on GitHub.
