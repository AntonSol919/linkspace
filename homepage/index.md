<div class="definition">
Supernet  [ˈsü-pərˌnet]<br>
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.
A communication protocol where the method of exchange is an extraneous concern.<br>
e.g. git, bitcoin, nostr, linkspace
</div>

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to control and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host which acts as the de facto administrator.

Linkspace is a supernet with the following highlights:

- Small API
- Fast packet processing (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressing
- Group/Domain split

[Basics](www.linkspace.dev/basics.html) gives a high level introduction.
Check out the [tutorials](www.linkspace.dev/docs/tutorial/index.html) to see an example of building an application.
For an overview of how the library functions fit together see the [Guide](www.linkspace.dev/docs/guide/index.html).
[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try and say hi.

The packet and database layout are stable, but some things are incomplete or undocumented.

Any feedback, questions, and ideas for improvements are welcome!

Of course the preferred way is to send a message to the test group.
For the less adventurous you can open an issue on GitHub.
