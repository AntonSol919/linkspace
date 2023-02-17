# The Linkspace Protocol - a general purpose supernet

> Supernet  [ ˈsü-pərˌnet ]
> A self-referential multi-user data organization protocol whose primary
> addressing method uses hashes instead of endpoint identifiers.
> A communication protocol where the method of exchange is a extraneous concern.
> e.g. git, bitcoin, nostr, the linkspace protocol

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.
Linkspace is supernet with with the following features:
- Small and powerfull API
- Fast ( Blake3, no json/b64 encoding, well aligned fields )
- Path ( url like ) addressable packets.
- group/domain split

To write applications for a supernet requires a different perspective compared to managing sockets.
The challenge is defining a super structure that can work across time and space with noisy incomplete data.

In return a supernet provides a lot of usefull properties including:
Serverless, lockin-resistant, lockout-resistant, extensible, scalable, accountable, privacy,
verifyable, (offline)availble, etc.

The packet format and index is stable, but expect some missing feature, rought edges, and the occasional bug.

I suggest you start with the docs/guide
