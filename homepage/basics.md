# Basics

## The TCP/IP internet

The TCP/IP internet is build on the following packet:

| Internet PACKET |
|-----------------|
| IP ADDRESS      |
| PORT            |
| DATA            |

Packets are transmitted, and eventually reach a destination `IP address`.
At the destination an application is listening on a `port`.

Packet don't arrive in order. (This is always the case in non-trivial networks.)
By adding a sequence number, we can order them at the destination.

The result is that conceptually each device is directly connected to every other device,
and applications can stream data to any other IP address.

Great for phone-calls or video streams.
To create more usefull tools we add various KeyValue systems on top.
i.e. systems where a query defines a set of keys and returns one or more value.

For example:

| System | Query                         | Value                 |
|--------|-------------------------------|-----------------------|
| DNS    | archive.org                   | 207.241.224.2         |
| HTTP   | /msg/hello.html               | <h1>Hello world!</h1> |
| SQL    | SELECT * from MSG where ID=1; | A message in a db     |

The thesis of this project is this:

We have reached the limits of this paradigm.

### The limit

First, are technical reasons. 
Things that are practically impossible. 
Which are better to [show](https://antonsol919.github.io/linkspace/docs/tutorial/) not tell.

Secondly, our culture w.r.t. the internet. 

Stream based systems connect to a central host.
The host provides the interface to their platform.
The interface acts as the 'guide' to the internet experience.

In the last two decades this relationship has ingrained a set of beliefs in the casual user.

- Tools/apps only work within a single ecosystem (corollary - external tools can/must have permission of the platform)
- Users should submit to companies that provide illiquid value on the condition of their loyalty.

Both have little to do with the reality of what is possible.
They exist as a story. A story that people get from the user interface provided by a platform.

With enough know-how you can process any data in any way from the internet, and those beliefs are just temporary frustrations. 

But because it requires much know-how, and many people do not belief it is in their power, we are limited in what we do.

There have been laws proposed to change the relationship between user and platform.
But asking the platforms to play nice will not break these limits. 

Instead, we need to change the idea at the core of the internet. 

## Linkspace

If IP/TCP provides streams for KeyValue systems, so users can talk _to_ server,
then linkspace provides a shared KeyValue space, so users can talk _about_ data. 

| IP Packet[^4] | Linkspace Point[^4]  | Notes                                    |
|---------------|----------------------|------------------------------------------|
|               | HASH                 | A unique packet ID (Blake3 hash)         |
| IP ADDRESS    | GROUP ID             | Set of recipients                        |
| PORT NUMBER   | DOMAIN               | Name chosen by App developer             |
| DATA          | DATA                 |                                          |
|               | ? Time Stamp         | Optional microseconds since 1970 (EPOCH) |
|               | ? PATH               | Optional Key with upto 8 components      |
|               | ? LINKS[]            | Optional list of (Tag, Hash)             |
|               | ? PUBKEY & SIGNATURE | Optional Public key and Signature        |

[^4]: Both have control fields that are irrelevant to a vast majority of developers.

In linkspace, a point is immutable data that is included in the hash.
A linkspace packet refers to all the bytes that are transmitted as a packet.
The packet includes the point fields, its hash, and a few mutable bytes to help send data across a group efficiently.

The hash is an automatically calculated number that is unique for each point.
The path is a custom defined key. For example /hello/world, or /sensors/temprature/left, or /webpages/index.html
Both hashes, and paths, can be used to query for packets.
The links in a packet are zero or more (tag, hash) tuples to reference other packets.

Using linkspace can be as simple as an IoT device broadcasting its state encoded as a linkspace packet every few moments.
Another device can append each message into a single file.

The `lk` CLI tool provides ways to deal with linkspace packets.
The sender can be as simple as: 

```bash 
LK_PASS=""
lk --init key
GROUP=\$( echo "My local group" | lk data | lk printf [hash:str] )
cat /dev/sensor/temperature/left | \ # pretend hardware
    lk keypoint myhome:$GROUP:/sensor/temperature/left -d'\n' | \ # signed packets at a made up domain,group,path
    lk collect --min-interval 10s mysensors:$GROUP:/sensor/temperature/left/bundle --chain-tag prev --sign | \ # link points together
    socat - UDP-DATAGRAM:255.255.255.255:24000,broadcast
```

In this case we create an indisputable log.
All current and future programs can unambiguously reference an entry and thus its data, links, etc.

The next issue is: how to make linkspace points available for more than the devices listening at the right moment?
Before we get there, lets briefly look at what other tools are in linkspace library:

- [Queries](https://antonsol919.github.io/linkspace/docs/guide/index.html#Query) are a standard to define a set of packets.
They are useful for filtering, selecting, and requesting packets.
- [ABE](https://antonsol919.github.io/linkspace/docs/guide/index.html#ABE) is a domain specific byte-templating language.
- A multi-reader single-writer database for packets.
- [Runtime](https://antonsol919.github.io/linkspace/docs/guide/index.html#Runtime) to wrap the database such that threads can match for old and new packets using queries.

To share linkspace points for other systems, we make a leap of logic into linkspace.
Instead of a defining a protocol as a stream of data, we define protocols as a functions over (new) points in linkspace.

Useful standards are defined as [conventions](https://antonsol919.github.io/linkspace/docs/guide/index.html#Conventions).
Packets with a well known meaning.
For instance, the [pull](https://antonsol919.github.io/linkspace/docs/guide/index.html#lk_pull) convention is a query saved as a point,
such that the application can notify a group exchange process to gather a set of packets.
[anyhost](https://antonsol919.github.io/linkspace/docs/guide/index.html#anyhost) is a minimal example that implements such a group exchange 
over TCP in a client-server architecture. 
Note that you can change or add exchange processes at will. 
They are an implementation detail depending on how points are exchanged and which can be saved where.

After setting up a linkspace group exchange, some complicated and impractical things are made simple.
Controlling the thermostat could be done by watching for a administrator key signing a control point.

```
ADMIN=$(cat admin)
lk pull myhome:$GROUP:/control/temperature -- pubkey:=:$ADMIN
lk watch myhome:$GROUP:/control/temperature -- pubkey:=:$ADMIN | \
    lk filter --live --bare -- create:>:[/or:[create]:[epoch]] | \ 
    lk printf [data] > /dev/sensor/control/temperature
```

We get authentication and traceability.
The magic starts when others get involved. 
For instance, an anonymous analysis can be as simple as rewriting each point with a new key and in the [#:pub] group, and link to the original without sharing that (`lk rewrite`).
A researcher can then unambiguously reference the data, and the code they used.

Linkspace is a work in progress and there are pieces missing.
If you want to give it a try, download the latest release and come say hi on the test group.
Or better yet, building something new by starting at the [tutorial](https://antonsol919.github.io/linkspace/docs/tutorial/).

## Is it worth the trouble?

On the plus side are the two issues mentioned previously.

Users are no longer locked as iliquid value conditioned on loyalty to a platform,
Tools can work on any packets, anywhere, anytime without the platform breaking them for the purpose of exclusivity.

Furthermore, things are made less complex in the long run.
The full stack required for a user experience is an order of magnitude thinner than it is now. (e.g. [mineweeper](https://antonsol919.github.io/linkspace/docs/tutorial/mineweeper/01.html))

Beyond the general resistance to change, I can see a few reasons to be skeptical.

### Reasons why it might be too much trouble.

- Too much overhead for an usecase.
  If all you want to do is stream one movie from a single host, and forget it then linkspace might be too much overhead.
  Few projects stay that simple. 
  Once you're adding: users accounts, IO error handling, user comments, sharding, etc.
  Linkspace becomes a thin solution for many requirements.
  It should also be fast enough for streaming video, `dd bs=10G count=1 if=/dev/zero | lk data > /dev/null` 
  and with blake3 it should be low energy to run on a potato phone. 

- People have to deal with more complexity.
  Semi true.
  Linkspace is lacking 3 decades of tools to make the web relatively easy.
  The nature of communication over distance makes it asynchronous distributed, so the complexity is not accidental.
  Furthermore, the number of configuration ends up smaller: Passwords, Groups, friends can be setup once and used by every application.
  Finally, giving people responsibility isn't a bad thing.
  Teaching people how to read, write, and do maths also gave them more responsibility. 
  It seems to me teaching people to fundamentally control a digital space is a logical step forward. 

- Isn't it a good thing that hosts administrate what others can do online?:
  A dumb illusion that will not survive this century. 

- Isn't it a good thing that hosts administrate what I can see online?:
  If you want to exclude spam or other unwanted data, this service can still be provided by a third party like it is today.

- Won't it devolve to the same paradigm of centralized systems?:
  Maybe, maybe not. If a users can walk away from a host platform without losing their history, the host has to give a better deal than they do now.


### Alternatives
There are similar systems, but they never felt complete to me. Reasons include:

- Only URLS and no hash addressing
- Only hash addressing but no custom keys (URL/Paths)
- To slow for most data (unaligned, JSON/B64 encoding steps). A system should be able to stream video, not just set up a different protocol to do so.
- No Group. Meaning there is no granularity in what you share.
- No domains. In my experience, without this separation a developer too daunted to hack something together.
- Overly complicated. (https://en.wikipedia.org/wiki/Conway%27s_law)
- Its not a supernet, but just a blockchain. A global sequence of signed packets with a link to previous entries, where the first users should be paid with the money of new people joining, and there is no place for human trusts.
- The wrong order of defining the system's components:
   - A fixed method of exchanging data, instead of a external/modular system. 
   - An ever growing set of auxiliary baked in stream protocols to negotiate a state. Expanding the system should have as few external dependencies as possible. i.e. defined building a request, requesting a human readable name, adding group members, should be encoded as points. (This doesn't 'solve' any inherent problem but it promotes dogfooding, replaceable designs when (gradually) introducing the system to a stack)
