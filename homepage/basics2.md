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

| System | Query                                | Value                                    |
|        |                                      |                                          |
| DNS    | archive.org                          | 207.241.224.2                            |
| HTTP   | /msg/hello.html                      | <h1>Hello world!</h1>                    |
| SQL    | SELECT * from MSG where ID=1;        | A message in a db                        |

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
|               |                      |                                          |
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
cat /dev/sensor/temperature/left | \
    lk keypoint mysensors:[#:lan]:/sensor/temprature/left --newline | \ 
    lk collect --min-interval 10s mysensors:[#:lan]:/sensor/temprature/left/collection --chain-tag prev | \ 
    socat - UDP-DATAGRAM:255.255.255.255:24000,broadcast 
```

In this case we create an indisputable log, and each event becomes useful in many more ways. 
Every other program can unambiguously reference each entry and thus its content.

The linkspace library is a broader set of tools for more advanced use cases.

- [Queries](https://antonsol919.github.io/linkspace/docs/guide/index.html#Query) are a standard to filter/select/request packets.
- [ABE](https://antonsol919.github.io/linkspace/docs/guide/index.html#ABE) is a domain specific byte-templating language. 
- [Runtime](https://antonsol919.github.io/linkspace/docs/guide/index.html#Runtime) is a database shared between multiple processes where each thread 
can read old and new packets matching a query. 

On top of these tools there exists [conventions](https://antonsol919.github.io/linkspace/docs/guide/index.html#Conventions).
Conventions are packets with a well known meaning.
For instance, the [pull](https://antonsol919.github.io/linkspace/docs/guide/index.html#lk_pull) convention is used by
an application to notify a group exchange process to gather a set of packets.

If you're convinced, download the latest release and come say hi on the test group, or give the 
[tutorial](https://antonsol919.github.io/linkspace/docs/tutorial/) a try.

For the more skeptical: 

## Is it worth the trouble? 

On the plus side, a lot of things can be made more simple in the long run.

Users are no longer locked as iliquid value conditioned on loyalty to a platform,
tools can work on any packets, anywhere, anytime without the platform breaking them for the purpose of exclusivity.

A full stack required for a user experience is an order of magnitude thinner than it is now. (e.g. [mineweeper](https://antonsol919.github.io/linkspace/docs/tutorial/mineweeper/01.html))

There are challenges. 
Beyond the general resistance to change, I can see a few reasons to be skeptical.

### Reasons why it might be too much trouble.

- Too much overhead for an usecase.
  If all you want to do is stream one movie from a single host, and forget it then linkspace might be too much overhead.
  Few projects stay that simple. 
  Once you're adding: users accounts, IO error handling, user comments, sharding, etc.
  Linkspace becomes a thin solution for many requirements.
  It should also be fast enough for streaming video, `dd bs=10G count=1 if=/dev/zero | lk data > /dev/null` 
  and with blake3 it should be low energy to run on a potato phone. 

- Users have to deal with more complexity.
  True. 
  At the moment linkspace is lacking 3 decades of tools to make the web relatively easy. 
  But the number of configuration ends up smaller. Passwords, Groups, friends can be setup once. 
  Furthermore, complexity brings you control, and you can still outsource that.
  Giving users responsibility isn't a bad thing. 
  500 years ago, few could read, and a preacher had to do it for us.

- Isn't it a good thing to administrate what others can do online?:
  They can't. It is an artificial limit. 
  Some tech founders and authoritarian politicians love that this is the type of web that's popular.
  Unless you're heavily invested in either this paradigm is against your interest.
  
- Isn't it a good thing to administrate what I can see online?:
  Excluding spam or other unwanted data can still be provided by a third party.

- Won't it devolve to the same paradigm of centralized control?:
  Maybe, but maybe not. It is a huge leap forward if users have the option to walk away from a provider without losing their stuff. 

### Alternatives
There are similar systems, but they never felt complete to me. Reasons include: 

- Only URLS and no hash addressing
- Only hash addressing but no custom keys (URL/Paths)
- Slow (unaligned JSON and B64 encoding steps). A protocol should be able to stream video, not just set up a different protocol to do so.
- No Group. Meaning there is no granularity in what you share.
- No domains. In my experience, without this separation a new developer are daunted to hack something together.
- Overly complicated. (https://en.wikipedia.org/wiki/Conway%27s_law)
- It is a blockchain. A global sequence of signed packets with a link to previous entries, where the first users should be paid with the money of new people joining.
