# Basics

## The KeyValue/TCP/IP Internet

The Internet and Web as we know it is is build using the following packet format:

| Internet PACKET |
|-----------------|
| IP ADDRESS      |
| PORT            |
| DATA            |

Packets are transmitted, and eventually reach a destination `IP address`.
At the destination an application is listening for packets to be received on a `port`.

Packet don't arrive in the order they were sent. 
This is physically unavoidable in non-trivial networks.
By adding a sequence number, we can order them at the destination.

The result is that conceptually each device is directly connected to every other device.
Applications stream data to any other IP address.

This is ideal for phone-calls or video streams.
To create more useful and dynamic applications we add various KeyValue systems on top.
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

There are technical reasons, that I belief are best understood in comparison to [examples](https://antonsol919.github.io/linkspace/docs/tutorial/),
and there are cultural reasons.

Stream based systems connect to a central host.
The host provides an app or website to talk to their platform.
This acts as the 'guide' to the internet experience.

In the last two decades this relationship has ingrained a set of beliefs in people.

- Tools/apps only work within a single ecosystem (corollary - external tools can/must have permission of the platform to work)
- Users should submit to companies that provide illiquid value on the condition of their loyalty.

Both have little to do with the reality of what is possible.

With enough know-how you can process any data in any way. 
But because it requires this know-how, people tend to accept the story told by the platforms and we are all lesser for it.

In response to this, there have been laws proposed to change the relationship between user and platform.
For instance, demanding that users can export their data.
This is not a real solution.

Instead, linkspace is a project to change the idea at the core of the internet. 

## Linkspace

If IP/TCP provides streams for KeyValue systems, so we can talk _to_ server,
then linkspace provides a shared KeyValue space, so we can talk _about_ data. 

A unit in linkspace is called a point. Each point has data, some auxiliary fields, and they are hashed. 

| IP Packet[^4] | Linkspace Point    | Notes                            |
|---------------|--------------------|----------------------------------|
|               | HASH               | A unique packet ID (Blake3 hash) |
| IP ADDRESS    | GROUP ID           | Set of recipients                |
| PORT NUMBER   | DOMAIN             | Name chosen by App developer     |
| DATA          | DATA               |                                  |
|               | TIMESTAMP          | microseconds since 1970 (EPOCH)  |
|               | PATH               | Key with upto 8 components       |
|               | LINKS[]            | list of (Tag, Hash)              |
|               | PUBKEY & SIGNATURE | Public key and Signature         |

[^4]: Both IP packets and linkspace packets have control fields that are irrelevant to a vast majority of developers.

The auxiliary fields are essentially optional[^5], e.g. a point does not have to be signed.

[^5]: Optional is slightly misleading. There exist 3 types: datapoint, linkpoint, and keypoint. Missing fields default to a specifc value when required. For a full specification checkout the guide. 

A "linkspace packet" refers to the byte format when a point is transmitted.

The GroupID and Domain field are somewhat analogous to an address and port respectively. 
The path is a custom defined key: For example /hello/world, or /sensors/temprature/left, or /webpages/index.html
Such that applications can quickly get a point by its hash, or by their path key. 
Links are a list of (tag, hash) tuples. They can link to other points.
 
The `lk` CLI tool provides ways to build simple script that deal with linkspace packets.
There are also bindings for Rust and Python. 

Using linkspace can be as simple as an IoT device broadcasting its state encoded as a linkspace packet every few moments.
Another device can append each message into a single file.

A sender might write their temperature into linkspace as follows:

```bash 
LK_PASS=""
lk init 
lk key 
GROUP=\$( echo "My local group" | lk data | lk printf [hash:str] )
cat /dev/sensor/temperature/left | \ # pretend hardware
    lk keypoint myhome:$GROUP:/sensor/temperature/left -d'\n' | \ # signed packets at a made up domain,group,path
    lk collect --min-interval 10s mysensors:$GROUP:/sensor/temperature/left/bundle --chain-tag prev --sign | \ # link points together
    lk save
```

The result is an indisputable log.
All current and future programs can unambiguously reference any entry.

The next issue is: how to make linkspace points available for more than the devices listening at the right moment?
Before we get there, lets briefly look at what other tools are in linkspace library:

- [Queries](https://antonsol919.github.io/linkspace/docs/guide/index.html#Query) are a standard to define a set of packets.
They are useful for filtering, selecting, and requesting packets.
- [ABE](https://antonsol919.github.io/linkspace/docs/guide/index.html#ABE) is a domain specific byte-templating language.
- A multi-reader single-writer database for packets.
- [Runtime](https://antonsol919.github.io/linkspace/docs/guide/index.html#Runtime) to wrap the database such that threads can match for old and new packets using queries.

To share linkspace points for other systems, we make a leap of logic into linkspace.
That is, instead of defining a protocol implemented using a stream of data, we define protocols as a functions over (new) points in linkspace.

Useful standards are defined as [conventions](https://antonsol919.github.io/linkspace/docs/guide/index.html#Conventions).
They can read/write packets with the correct fields set to have a specific meaning.
For instance, the [pull](https://antonsol919.github.io/linkspace/docs/guide/index.html#lk_pull) convention creates a point at a specific location with a query in its data field. This is noticed by a group exchange process which will gather matching points from the group.

[anyhost](https://antonsol919.github.io/linkspace/docs/guide/index.html#anyhost) is a basic example that implements such a group exchange 
over TCP in a client-server architecture. 

Note that you can change or add exchange processes at will. 
The linkspace on top of which we build applications is independent of how the data is send between devices.
Adding redundant methods, and receiving a message twice does not effect the state.

Exchange is an implementation detail. It depends on your use-case: 'who can share what and where?'[^6].

[^6]: A convention for group membership w.r.t. public keys such that only accepted members can access a group is in the works. 

For our IoT device we can go a step simpler. Instead of `lk save` we can directly use `socat - UDP-DATAGRAM:255.255.255.255:6070,broadcast` and have 
a storage device on our LAN safe all the packets.

After a group exchange is configured, we can chagne our base model from processing streams into functions over state.
Some complicated and impractical things become less complex overall.

Controlling the thermostat could be done by watching for a administrator key signing a control point.

```bash
ADMIN=$(cat admin)
lk pull myhome:$GROUP:/control/temperature -- pubkey:=:$ADMIN
lk watch myhome:$GROUP:/control/temperature -- pubkey:=:$ADMIN | \
    lk filter --live --bare -- create:>:[/or:[create]:[epoch]] | \ 
    lk printf [data] > /dev/sensor/control/temperature
```

We get authentication and traceability in just a few lines of code.
The real magic starts when the data breaks outside of the uses we first imagined.

People or companies can gather, process, and expand on the points in linkspace.
Any analysis can unambiguously reference the original event without using an ad-hoc database and referencing scheme.
Making it anonymous[^7] can be achieved by doing a `lk rewrite` with a new key referencing the old data.
[^7]: Making it anonymous equally well as we do today. It is notoriously hard to truely make data anonymous.

Linkspace is a work in progress. There are pieces missing and still in development.
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
  Few projects stay that simple. Most projects grow in scope to identify users, save their comments, add them to groups, scale beyond a single server. 
  Once a full stack is build, linkspace can be a thin alternative.
  Furthermore, it is designed to be fast/low energy, such that you can stream a video on a potato phone. 
  `dd bs=10G count=1 if=/dev/zero | lk data > /dev/null` 

- Isn't it a good thing that hosts administrate what others can do online?:
  A dumb illusion that will not survive this century. 

- Isn't it a good thing that hosts administrate what I can see online?:
  If you want to exclude spam or other unwanted data, this service can still be provided by a third party like it is today.

- People have to deal with more complexity.
  Semi-true.
  The nature of communication over distance is autonomous and asynchronous, so the complexity compared to stream-based centralized hosts isn't accidental.
  Linkspace is also lacking 3 decades of tools that make the web relatively easy for users.
  Furthermore, the number of configuration ends up smaller: Passwords, Groups, friends can be setup once and used by every application.
  Finally, giving people responsibility isn't a bad thing.
  Teaching people how to read, write, and do maths also gave them more responsibility.
  It seems to me teaching people to fundamentally control a digital space is a logical next step,
  and to deny them this autonomy a recipe for disaster. 

- Won't it devolve to the same paradigm of centralized systems?:
  Maybe, maybe not. If a users can walk away from a host platform without losing their history, the host has to give a better deal than they do now.


### Alternatives
There are similar systems, but they never felt complete to me. Reasons include:

- Only URLS and no hash addressing.
- Only hash addressing but no custom keys (URL/Paths).
- Too slow for most data (unaligned, JSON/B64 encoding steps). A system should be able to stream video, not just set up a different protocol to do so.
- No Group. Consequently there is no or little granularity in what you share.
- No domains. Consequently the barrier to quickly hack something together is high when you're unsure if you're stepping on someones toes.
- [Conway's law](https://en.wikipedia.org/wiki/Conway%27s_law).
- Its not a full supernet, but just a blockchain. (A singular log of packets with links to its previous entry that hold accounting data)
- The wrong order of defining the system's components. 
   - A fixed method of exchanging data, instead of a external/modular system. 
   - An ever growing set of stream protocols to negotiate a state.
