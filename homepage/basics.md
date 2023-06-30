# Basics

## The internet of streams

The digital world you know is build on structuring streams of data.
At its core the internet[^2] attempts to provide a model where: 

For any two connected devices running any application, there exists a connection to transmit data.

[^2]: TCP/IP - I'll be loose on definitions and oversimplify a lot. I don't expect readers to know or care for the details. But if you find an incorrect statement shoot me a message.

To do this it uses the following types of packets to transmit data between two devices.

:::{.container}
+-----------------+-----------------------+-----------------------------------------------------------------+
|                 | Field                 | Purpose                                                         |
+=================+=======================+=================================================================+
| Internet Packet | IP ADDRESS            | Address for a device                                            |
+                 +-----------------------+-----------------------------------------------------------------+
|                 | PORT                  | A number to address an application on a device                  |
+                 +-----------------------+-----------------------------------------------------------------+
|                 | SEQUENCE ID           | A number to (re)order packets when they arrive out of order     |
+                 +-----------------------+-----------------------------------------------------------------+
|                 | DATA                  | Set by application                                              |
+-----------------+-----------------------+-----------------------------------------------------------------+
:::

Packets carrying data are transmitted, and using the `IP address` they eventually reach their destination; a computer such as you're using right now.
On the computer an application is running that looks for packets with a specific `port` to arrive.

In transit, a packet can get lost or corrupted.
The result is that packets don't arrive in the order they were sent.
By adding a sequence number, the destination can verify all packets have arrived and reorder them to the order they were sent in.

I imagine the prototype internet was first discovered when it was realized that packet SEQUENCE IDs are essential (i.e. unavoidable) complexity. When transmitting data something must define the order of packets. Consequently the physical route each packet takes is irrelevant.

The result is that _conceptually_ each application on each device can talk to any other application on any other device.

This model is ideal for phone-calls or video streams.
To build more interesting applications we create protocols to add further structure to the stream of data.
There are thousands of different protocols, but what most of them have in common is a way to transmit questions and answers.

A couple of well known internet protocols that have this property are:

| System | Question                      | Answer              |
|--------+-------------------------------+---------------------|
| DNS    | archive.org                   | 207.241.224.2       |
| HTTP   | /forum/index.html             | Hello world!        |
| FTP    | /Projects/linkspace/readme.md | In a supernet [...] |
| SQL    | SELECT * from MSG where ID=1; | A message in a db   |

Why you should consider using linkspace is this:

<b>We have reached the limit of this streaming questions/answers paradigm.</b>

There are many difficulties with it.

As with most abstraction, the details leak.
Streams get disconnected, the other side hangs up, the other side is overloaded, etc.

The leaks compound when more than two computers are involved.
Answers become invalid, integrity is build ad-hoc, backups require additional logic, a shared state requires expensive synchronization[^sync], etc.

[^sync]: Or instead of some form of locking you add unique ID's to all your event in the network. If you chose a strong hash, and each event can reference others by their hash - then you've essentially built a special purpose supernet.

Once you start thinking in terms of supernets, it becomes clear that this is accidental complexity.
If we take a different approach to two or more computers communicate, these concerns become irrelevant or trivial - and we can do things that are practically impossible if we keep talking streams.

## Linkspace

Linkspace attempts to provide a model where: for any group running any application, there exists a space to address data[^idea].

[^idea]: This idea and other ideas used in linkspace aren't new. But I believe linkspace is a simple and powerful synthesis compared to previous attempts (see [alts](#alts)).

If the current internet is essentially streams for key-value systems, so you can talk _to_ server, 
then linkspace is essentially a shared key-value space, so groups can talk _about_ data.

A unit in linkspace is called a **point**. Each point has data, some auxiliary fields, and is uniquely identified by its hash.

To understand what each field does lets start with a simple example of a message forum. 


:::{.container .pkt .pd}
+-----------------------------------+----------------------------------+
| Key                               | Value                            |
+===================================+==================================+
| /image/BrokenMachine.jpg          | [image data]                     |
+-----------------------------------+----------------------------------+
| /thread/Coffee machine broke!/msg | Fix pls? image/BrokenMachine.jpg |
+-----------------------------------+----------------------------------+
:::


<div class="op">+</div>

:::{.container .pkt .pd}
+-----------------------------------+----------------------------------+
| /thread/Can we use Rust?/msg      | I heard it is great.             |
+-----------------------------------+----------------------------------+
:::

<div class="op">=</div>

:::{.container .pkt .pd}
+-----------------------------------+----------------------------------+
| /image/BrokenMachine.jpg          | [image data]                     |
+-----------------------------------+----------------------------------+
| /thread/Can we use Rust?/msg      | I heard it is great.             |
+-----------------------------------+----------------------------------+
| /thread/Coffee machine broke!/msg | Fix pls? image/BrokenMachine.jpg |
+-----------------------------------+----------------------------------+
:::


The key "image/BrokenMachine.jpg" is called a **path** and maps to [image data].
So far this should look familiar as it is similar to files in directories.
I'll refer to each entry as a **point**, and multiple entries as a **set**.
The example shown has two sets **merging**. The result is a new set with 3 messages.

One of the most useful aspects of linkspace is a way to talk and think about digital communication in terms of a set of points instead of connections.

Practically any digital communication can be understood as merging sets of points.

Online platforms have dubbed different words for actions you can take:
'_creating posts_', '_uploading image_', '_upvote/like a post_', '_stream a video_', etc.
Fundamentally they can be understood as a frontend application providing an interface to __merge__ sets of points.
Either on your computer or on their computer[^device].

[^device]: Your computer immediatly forgetting those data points is a configuration detail.

The internet we use today has a single host design. 
For instance, a web-browser or app contacts `http://www.some_platform.com`
for the key `/image/BrokenMachine.jpg` to get an image.

This is simple, but it has downsides.

There is a misconception on what an address is[^address],
a host can get disconnected,
you can't (re)share and (re)use your copy of the data,
and there is no standard on what happens when two people create two different `image/BrokenMachine.jpg` but with different pictures.

[^address]: The perception is created that the address 'http://www.some_platform.com/image/BrokenMachine.jpg' is addressing '[image data]' - this is wrong. The address is used for your request to find where it needs to go, this address then usually replies with '[image data]'. A subtle but consequental difference. Linkspace does not have this discrepency.

I would argue these fall under accidental complexity.

Especially the last one. Once the speed of light is measurable in a network, it requires a specific design to avoid two or more computers to write to the same path.

In our single host design, the data is hosted on a server and the person who has administrative access to that server can then administrate which one is the 'real' copy, and which one should be forgotten. 

In linkspace there is no such thing as a 'real' copy on a single host.

Every path can refer to multiple points.

Each point is hashed.
i.e. there exists a unique 32 bytes (or ~77-digit number) that uniquely identifies the point[^uniq] (which I'll show as <span id="hh0" >[HASH_0]</span> instead of typing out).

[^uniq]: Unique for all intents and purposes - except for the purpose of counting to 2^256+1.

It doesn't matter when or where sets are merged - the result only has a single copy per message.

:::{.container .pkt .phd}
+-----------------------------------+--------------------------------+----------------------------------+
| /image/BrokenMachine.jpg          | <span id="hh0">[HASH_0]</span> | [image data]                     |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Coffee machine broke!/msg | <span id="hh1">[HASH_1]</span> | Fix pls? image/BrokenMachine.jpg |
+-----------------------------------+--------------------------------+----------------------------------+
:::

<div class="op">+</div>

:::{.container .pkt .phd}
+-----------------------------------+--------------------------------+----------------------------------+
| /image/BrokenMachine.jpg          | <span id="hh0">[HASH_0]</span> | [image data]                     |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Emacs or vim?/msg         | <span id="hh2">[HASH_2]</span> | I heard they're better than VS   |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Emacs or vim?/msg         | <span id="hh3">[HASH_3]</span> | Emacs with vim bindings ofcourse |
+-----------------------------------+--------------------------------+----------------------------------+
:::

<div class="op">=</div>
 
:::{.container .pkt .phd}
+-----------------------------------+--------------------------------+----------------------------------+
| /image/BrokenMachine.jpg          | <span id="hh0">[HASH_0]</span> | [image data]                     |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Emacs or vim?/msg         | <span id="hh2">[HASH_2]</span> | I heard they're better than VS   |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Emacs or vim?/msg         | <span id="hh3">[HASH_3]</span> | Emacs with vim bindings ofcourse |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Coffee machine broke!/msg | <span id="hh1">[HASH_1]</span> | Fix pls? image/BrokenMachine.jpg |
+-----------------------------------+--------------------------------+----------------------------------+

:::

We can uniquely get a specific point by its <span id="hh0">[HASH_0]</span>,
or multiple entries through a path "/thread/Tabs or spaces/msg".

This might seem more trouble than existing solutions like a filesystem or HTTP.
In those, one request by name gets you a single result. 

However, this is not a real issue for two reasons. 

In practice it is trivial to only request 'the latest value' or 'the latest value signed by someone you trust'.

The later is especially interesting. 
If an application only requests points signed by a specific key, it effectivly administrates similar to how it is done in our current single host-administrator design.
However, it has the additional property that it can be independent from hosting the data. 
i.e. in linkspace 'hosting data' and 'content administration' can be decoupled.

:::{.container .pkt .pkthd}
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
| /image/BrokenMachine.jpg          | [@:alice:salesexample]      | 2015/01/29 | <span id="hh0">[HASH_0]</span> | [image data]                                |
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
| /thread/Coffee machine broke!/msg | [@:alice:salesexample]      | 2023/03/02 | <span id="hh1">[HASH_1]</span> | Fix pls? image/BrokenMachine.jpg            |
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
:::

<div class="op">+</div>

:::{.container .pkt .pkthd}
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
| /thread/Coffee machine broke!/msg | [@:bob:maintenance:example] | 2023/03/02 | <span id="hh3">[HASH_4]</span> | Hey <span id="hh1">[HASH_1]</span>!         |
|                                   |                             |            |                                | Isn't <span id="hh0">this</span> from 2015? |
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
:::

<div class="op">=</div>

:::{.container .pkt .pkthd}
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
| /image/BrokenMachine.jpg          | [@:alice:salesexample]      | 2015/01/29 | <span id="hh0">[HASH_0]</span> | [image data]                                |
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
| /thread/Coffee machine broke!/msg | [@:alice:salesexample]      | 2023/03/02 | <span id="hh1">[HASH_1]</span> | Fix pls? image/BrokenMachine.jpg            |
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
| /thread/Coffee machine broke!/msg | [@:bob:maintenance:example] | 2023/03/02 | <span id="hh3">[HASH_4]</span> | Hey <span id="hh1">[HASH_1]</span>!         |
|                                   |                             |            |                                | Isn't <span id="hh0">this</span> from 2015? |
+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
:::

A point has two preceding fields. A **group** that signal who can read/write to the tree, and a **domain** field to indicate which application should read it.
Essentially any pair of (domain, group) has its own tree.

For example the `msg_board` application and the `[#:example]` group.

:::{.container .pkt .dgpkthd}
+----------+-------------+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
|msg_board | [#:example] | /image/BrokenMachine.jpg          | [@:alice:salesexample]      | 2015/01/29 | <span id="hh0">[HASH_0]</span> | [image data]                                |
+----------+-------------+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
|msg_board | [#:example] | /thread/Coffee machine broke!/msg | [@:alice:salesexample]      | 2023/03/02 | <span id="hh1">[HASH_1]</span> | Fix pls? image/BrokenMachine.jpg            |
+----------+-------------+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
|msg_board | [#:example] | /thread/Coffee machine broke!/msg | [@:bob:maintenance:example] | 2023/03/02 | <span id="hh3">[HASH_4]</span> | Hey <span id="hh1">[HASH_1]</span>!         |
|          |             |                                   |                             |            |                                | Isn't <span id="hh0">this</span> from 2015? |
+----------+-------------+-----------------------------------+-----------------------------+------------+--------------------------------+---------------------------------------------+
:::

Note that the group and public keys are actually 32 bytes. 
For brevity I used the [LNS](./lns.html) representation for `[@:alice:salesexample]` and `[#:example]`.
LNS solves a similar problem as DNS, i.e. turning a name like `www.example.com` into a number like `192.168.0.1`.

Finally, the messages we used as an example have a <span id="hh1">[HASH]</span> directly in their data field.
This would not work well for most use-cases.
Instead, a point in linkspace has a list of [links](./docs/guide/index.html#lk_linkpoint) adjacent to the data.

:::{.container}
+---------------------+----------------------+-------------------------------+--------------------+
|                     | Field[^4]            | Purpose                       | IP Packet Analogue |
+=====================+======================+===============================+====================+
| Linkspace Point     | HASH                 | A unique ID (Blake3)          |                    |
+                     +----------------------+-------------------------------+--------------------+
|                     | GROUPID              | Set of recipients             | IP ADDRESS         |
+                     +----------------------+-------------------------------+--------------------+
|                     | DOMAIN               | Name chosen by App developer  | PORT NUMBER        |
+                     +----------------------+-------------------------------+--------------------+
|                     | DATA                 | Set by application            | DATA               |
+                     +----------------------+-------------------------------+--------------------+
|                     | TIMESTAMP            | Microseconds since 1970-01-01 |                    |
+                     +----------------------+-------------------------------+--------------------+
|                     | PATH                 | Key to look up                |                    |
+                     +----------------------+-------------------------------+--------------------+
|                     | LINKS[]              | list of (Tag, Hash)           |                    |
+                     +----------------------+-------------------------------+--------------------+
|                     | PUBKEY & SIGNATURE   | Optional - identifies creator |                    |
+---------------------+----------------------+-------------------------------+--------------------+
:::

[^4]: Both TCP/IP packets and linkspace packets have control fields that are irrelevant to a vast majority of developers.

For the full specification of creating and writing points see the [guide](./docs/guide/index.html#packet_layout)

There are some nuances and various advanced topics.
However, this should be enough to reason about the basics:

Users generate their identity, together they form groups and set up a method to exchange data.

The result is that _conceptually_ an application only needs to process the state of the trees.

## Ready to give it a try?

Linkspace is not an end-user application.
It is a software library and command line tools.
A GUI frontend to manage groups/domains/keys is outside its scope.

The packet format is stable. Points created will stay readable in future versions.
The API is mostly stable but will have some breaking changes and additional conventions.

To give it a try you can [Download](https://github.com/AntonSol919/linkspace/releases) the pre-build CLI and python bindings to follow along with
the [tutorial](./docs/tutorial/index.html) or the more technical [Guide](./docs/guide/index.html),

For the adventurous there is initial support for wasm, and a POC HTTP bridge called [Webbit]( https://github.com/AntonSol919/webbit) that works similar to WebDAV.

# Q&A

### Who is linkspace for?

Me from 10 years ago, me today, the next generation that wants their internet to be better, and everyone else. In that order. 

What i wished for 10 years ago i have now.

- I can define, prototype, build, and run (real) serverless offline-first apps quickly.
- I can build an application without having to decide how to administrate for everyone that uses it - that responsibility sits with the (group of) users.

To the next generation i'll say this. 

The single greatest insanity of this time, and its defining feature, is that a few for-profit advertisement / propaganda services are the host-administrators of the digital 'public squares'.

It is questionable for people (and their group) to be subjected to the interests of an unacountable administrator.
But to me the real danger is the medium as the message: You are not in control, surrender your minds for profit and submit to apathy.
The wasted potential for people to learn and grow is staggering. We are all lesser for it.

I wish for the message of linkspace to be: You are in control, and you should help build the best place you can.


### Is linkspace a blockchain?

No.

Blockchains and supernets share a common idea:

Using cryptographic hashes and public keys to provide a model for communication data that transcends the connection used to share it.

Blockchain use the cryptography to batch multiple events into 'blocks'. As new blocks are made they are linked or 'chained' to the previous block.
This creates a consensus of all events that happened.

A blockchain's goal is to provide a model and tools to make consensus simple.

However, consensus on all events isn't that useful as a foundational feature for the vast majority of applications.

Supernets don't bother with a global truth. Their goal is to work with the links between packets.
Consequently, it's not difficult to define a 'blockchain' style consensus in a general purpose supernet.

### Won't an app have a lot of overhead compared to a basic Web server?

If all you want to do is stream one movie from a single host and forget it, then linkspace might be too much overhead.
Few projects stay that simple. Most projects grow in scope: to identify users, save their comments, add them to groups, scale beyond a single server, etc.

Once a full stack is build, linkspace is very small w.r.t. its features.

As far as overhead goes, it is designed to be fast/low energy such that a low-end phone can use it to stream video.

### Can you ask people to deal with the added complexity?{#complexity}

Yes.

Linkspace lack 6 decades of tooling that made the internet and web relatively easy for users, but that can change.

But is it worth it?

Yes.

Technically supernets better model the reality of multi party communication - asynchronous and authenticated[^auth]
In the long run they could end up with less moving parts and with fewer configurations.

[^auth]:Authenticated as in: cryptographicaly proven that messages were created by a user of a public key regardless how you got the message - I call this 'the reality' because a wire-dump of an HTTPS session is also proof that the keyholder send the message.

For society it is important that anyone can take responsibility if they want.

### Isn't it a good thing that a host administrates what I and others see online?

I agree it is not categorically a bad thing, especially in the 'public square'.

There are different ways to do so, the straight forward approach is:

- An application can have you trust the public key of third party service to whitelist content. Emulating the current system of 'admins', while still having users give the option to replace them.

Alternativily you can have friends and friends of friends vouch for content. 
I suspect the latter to become more important as AI drives the cost of bullshit to zero and platforms can't keep up.

### Won't we end up with the same paradigm of highly centralized control?

Maybe, maybe not.
If user today could walk away from a host-administrator without losing their history, identity, and links to others; 
then they would get a better deal then they do now.


## Why not [alternative]?{#alts}

There are two types of systems I'm certain aren't the right building blocks for the next digital era.

- Synchronizing chain of trust: Is slow and not useful for the vast majority of users - (and easy to emulate in a supernet).
- Faster email/Activity Pub : It's not that fast and it is using server-defined authenticity[^i] - plus most of the list below.

[^i]: AFAIK content hashing and publickey identities could one day be the default - but it seems to be extremely complex to build on, and apps will miss out on much of the benefit of a supernet if its optional or overly generic.

Other supernet-like systems are limited in some way or simply choose a different design:

- Too specialized. For example, a system like Git has a lot of plumbing for diffing each commit.
- It has either hashes or pre defined path, not both as first class addresses.
- Too slow. Packet routing/parsing should be doable in just a few instructions - ideally possible in an integrated circuit. It should be fast enough to stream video without using a second protocol. That means no json or base64.
- No Groups. Setting who you share with and how is not supported or only supports 'run multiple instances'.
- No domains. Everything becomes one app with premature-bureaucracy that can grind development/experiments to a halt.
- Its focused on signatures and consensus.
- Large 'packets' - a hash might refer to gigabytes. This requires multiple levels to deal with fragmentation in multiple ways.
- Poor scripting support. 
- Excessively interwoven components. e.g. Transmitting packets require a fully running 'node' with a fixed method of exchanging or saving data. 

That does not mean I think alternatives are necessarily worse.
Different systems have different strong points.

