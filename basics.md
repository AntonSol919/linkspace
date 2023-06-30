# Basics

## The internet of streams

The current internet we're using is an internet of streams.
The internet[^2] attempts to provide a model where: for any two connected devices running any application, there exists a connection to transmit data.

[^2]: TCP/IP - I'll be loose on definitions and oversimplify a lot. I don't expect readers to know or care for the details. But if you find an incorrect statement shoot me a message.

It presents a direct line to anyone.

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

Packets carrying data are transmitted, and each hop moves them closer until they reach the destination `IP address`, i.e. a computer such as you're using right now.
On the computer an application is listening for packets with a specific `port` to be received.

In transit, a packet can get lost or corrupted.
Consequently, packets don't arrive in the order they were sent.
By adding a sequence number, we can reorder them at the destination[^telephone].

[^telephone]: I imagine the prototype internet was first discovered when it was realized that packet SEQUENCE IDs are essential complexity for data integrety, and that consequently the physical route each packet takes is irrelevant.

The result is that _conceptually_ each application on each device can talk to any other application on any other device.

This model is ideal for phone-calls or video streams.
To build more interesting applications we create protocols to add further structure to the data.
There are thousands of different protocols, but what most have in common is that they transmit questions and answers.
Or in other words: queries and responses, or keys and values[^jargon].

This creates a mapping between input and output.

[^jargon]: In some situations there is a technical difference between "query-response" and "key-value" systems. When describing them in a network (where origin and time are implied) they are indistinguishable.

A couple of well known protocols that provide a mapping between keys and values over the internet are:

| System | Query                         | Value               |
|--------+-------------------------------+---------------------|
| DNS    | archive.org                   | 207.241.224.2       |
| HTTP   | /forum/index.html             | Hello world!        |
| FTP    | /Projects/linkspace/readme.md | In a supernet [...] |
| SQL    | SELECT * from MSG where ID=1; | A message in a db   |

The reason for building linkspace is this:

<b>We have reached the limit of this paradigm.</b>

As with most abstraction, the details leak.
Streams get disconnected, the other side hangs up, the other side is overloaded, etc.
In multi party systems with key-value stores these leaks reach further.
Key-value stores become inconsistent (especially common in caching), database corruption, and they require expensive synchronization between devices[^sync].

[^sync]: Or instead of synchronization you add unique ID's to each event across the network. If you chose a strong hash, and each event can reference others by their hash - you've just build a supernet.

But I think the best way to make the point for reaching the limit, is from the perspective of an alternative.

## Linkspace

Linkspace attempts to provide a model where: for any group running any application, there exists a space to address data[^idea].

[^idea]: This idea and other ideas used in linkspace aren't new. But I think linkspace is a simple and powerful synthesis compared to previous attempts (see [alts](#alts)).


If the current internet provides streams for key-value systems, so we can talk _to_ server,  
then linkspace provides a shared key-value space, so we can talk _about_ data.

A unit in linkspace is called a **point**. Each point has data, some auxiliary fields, and is uniquely identified by its hash.

Before listing each field i'll motivate them with an example.

### Merging sets

Imagine building a message forum.


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
This currently looks just like your files and directories.
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
for the key `/image/BrokenMachine.jpg` to get their data.

This is simple, but it has downsides.

There are common misconceptions on what an address is[^address].
A host can get disconnected,
you can't (re)share and (re)use your copy of the data,
and every host has to pick a strategy when you merge two sets but they share the same path.

[^address]: The perception is created that the address 'http://www.some_platform.com/image/BrokenMachine.jpg' is addressing '[image data]' - this is wrong. The address is used for your request to find where it needs to go, this address then usually replies with '[image data]'. A subtle but a consequental difference. Linkspace does not have this discrepency.

I would argue these are all accidental complexity.

Most noticeably the last one: How to merge two sets if both have `thread/BrokenMachine.jpg` but different data.
Once the speed of light is measurable in a network, it is unavoidable for two computers to write to the same path without a costly synchronization steps.

In linkspace there is no such thing as a 'real' copy on a single host.

Every path refers to multiple values.

Each point is hashed.
i.e. there exists a unique 32 bytes (or ~77-digit number) that uniquely identifies the point[^uniq] (which I'll show as <span id="hh0" >[HASH_0]</span> instead of typing out).

[^uniq]: Unique for all intents and purposes - except for the purpose of counting to 2^256+1.

Therefor it doesn't matter when or where sets are merged - and they only leave a single copy when both have the same message.

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

A point also has a creation date and are **optionally** signed - such that you can identify who created it.

As such, we can uniquely get a specific point by its <span id="hh0">[HASH_0]</span>,
or multiple entries through a path "/thread/Tabs or spaces/msg".

This might seem more trouble than existing solutions like a filesystem or HTTP.
In those key-value systems a single path gets you a single result.

However, in practice it's trivial to behave similarly by adding constraints to a requested set; Such as 'only return the latest', or 'the latest signed by a specific public key'.

Conversely, I would argue both filesystems and HTTP servers are more trouble over all.
They hide that they return multiple values - a new value depending on when and where you make the request.

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

<div class="op">+</div>

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

For the full layout of packets see the [guide](./docs/guide/index.html#packet_layout)

There are some nuances and various advanced topics such as: Paths can be any bytes, the [queries](./docs/guide/index.html#Query) syntax for defining sets, and the convention on requesting subsets of data in a group by [pulling](./docs/guide/index.html#lk_pull).

However, I hope this gives you enough to reason about the basics:

Users generate an identity, groups set up a method to exchange data.

The result is that _conceptually_ an application only needs to process the state of the trees.

## Give it a try?

The linkspace library is beta software.

The packet format is stable. Points you create will stay readable in future versions.
The library API is missing some features and will have some breaking changes.

However, there aren't tools yet to make things simple.

You can [Download](https://github.com/AntonSol919/linkspace/releases) the pre-build CLI and python library to follow along with
the [tutorial](./docs/tutorial/index.html) or the more technical [Guide](./docs/guide/index.html),
and say hi on the test group.

# Q&A

### Is linkspace a blockchain?

No.

Blockchains and supernets share a common idea:

Using cryptographic hashes and public keys to provide a model of data that trancends the connection used to share it.

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

Supernets better model the reality of multi party communication - asynchronous and authenticated[^auth]

[^auth]:Authenticated as in: cryptographicaly proven that messages were created by a user of a public key regardless how you got the message - I call this 'the reality' because a wire-dump of an HTTPS session is also proof that the key holder send the message.

In the long run they could end up with less moving parts and with fewer configurations.

Also important is that anyone can take responsibility.
It is not without danger for billions of people to spend hours each day in a paradigm where they can not take back control over systems they consider the 'public square'.

My hope is to look back at this time as the era of digital fiefdoms.
The next era should balance out the influence of host-administrators,
and together people can define what a 'real' copy is.

### Isn't it a good thing that a host can administrate what I and others see online?

I agree it is not categorically a bad thing.

To have an administrator that filters the digital space there are a multiple options. To name a two:

An application can have you trust the public key of third party service to whitelist content. Effectively emulating the current system of 'admins', while still having users give the option to replace them.

Or an application can have you trust only signatures from friends or friends of friends.
(A system that will become more important as AI drives the cost of bullshit to zero.)

### Won't we end up with the same paradigm of centralized control?

Maybe, maybe not.
If a user could walk away from a host server without much trouble, the host has to give a better deal than they do now.

## Why not [alternative]?{#alts}

There are two types of systems I'm positive aren't the right building blocks for the next digital space.

- Synchronizing chain of trust: Is slow and not useful for the vast majority of users.
- Faster email: (like ActivityPub) is just faster email - without being very fast - plus most of the list below.

Other supernet-like systems are limited in some way or simply choose a different design:

- It has either hash addresses or custom url addresses, not both.
- Too slow. Packet routing/parsing should be doable as very few instructions - ideally doable in silicon. It should be fast enough to stream video without using a second protocol. That means no json.
- No Groups. Limiting who you share with is not supported - apps can't be used in a private group.
- No domains. Everything becomes one app.
- Its focused on signatures and consensus.
- Large 'packets' - a hash might refer to gigabytes. This requires multiple levels to deal with fragmentation in multiple ways.
- Poor (bash) scripting/piping support
- A blessed/fixed method of exchanging data - either it fits your use-case or you're out of luck.

With linkspace I believe I've found a good structure for 'the stack' required to build a general purpose supernet.
Linkspace is first and foremost its packet format, and secondly a bunch of tools to wrangle them to fit a use-case.

That does not mean I think alternatives are necessarily worse or useless.
Different systems have different strong points.
