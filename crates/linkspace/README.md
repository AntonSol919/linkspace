# Linkspace - a general purpose supernet

> Supernet  [ˈsü-pərˌnet]
> A self-referential multi-participant data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is an extraneous concern.
> e.g. git, bitcoin, nostr, linkspace

Linkspace combines the core ideas of HTTP and git.
It is a supernet. A protocol where we talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to control and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host which acts as the de facto administrator.

[Linkspace](https://antonsol919.github.io/linkspace/index.html) is supernet with the following highlights:

- Small API
- Fast packets (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressing
- Group/Domain split

[Basics](https://antonsol919.github.io/linkspace/index.html#basics) gives a high level introduction.
Check out the [tutorials](https://antonsol919.github.io/linkspace/docs/tutorial/index.html) to see an example of building an application.
For a technical description from first principles see the [Guide](https://antonsol919.github.io/linkspace/docs/guide/index.html).
[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try and say hi.

The packet and database layout are stable, but some things are incomplete or undocumented.

Any feedback, questions, and ideas for improvements are welcome!

Of course the preferred way is to send a message to the test group.
For the less adventurous you can open an issue on GitHub.
