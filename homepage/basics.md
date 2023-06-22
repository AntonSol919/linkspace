# Basics

## The internet of streams

The internet[^2] attempts to provide a model where: for any two connected devices running any application, there exists a connection to transmit data.

[^2]: TCP/IP - I'll be lose on definitions, as i don't expect all readers to know the details. But if you find a real incorrect statement shoot me a message.

It is a direct line to anyone.

To do this it uses the following types of packets.

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

Packets carrying data are sent and will eventually reach a destination `IP address`, i.e. a device such as this one you're reading on.
On this device an application is listening for packets with a specific `port` to be received.

In transit, a packet can get lost or corrupted.
Consequently, packets don't arrive in the order they were sent.
By adding a sequence number, we can reorder them at the destination[^telephone].

[^telephone]: I imagine the prototype internet was first discovered when it was realized that packet SEQUENCE ID's are essential complexity for data integrety, and that consequently the physical route each packet takes is actually irrelevant.

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

<b>We can't progress if we stay in this paradigm.</b>

It think it is better to explain the alternative than argue this point.

## Linkspace

Linkspace attempts to provide a model where: for any group running any application, there exists a space to address data.

If the current internet provides streams for key-value systems, so we can talk _to_ server,  
then linkspace provides a shared key-value space, so we can talk _about_ data.

This and other ideas in linkspace aren't new. But I believe linkspace offers a simple and powerful synthesis of ideas compared to other attempts (see [alts](#alts)).

A unit in linkspace is called a **point**. Each point has data, some auxiliary fields, and is uniquely identified by its hash.

:::{.container}
+---------------------+---------------------+-------------------------------+--------------------+
|                     | Field[^4]           | Purpose                       | IP Packet Analogue |
+=====================+=====================+===============================+====================+
| Linkspace Point     | HASH<sub>32</sub>   | A unique ID (Blake3)          |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | GROUP ID            | Set of recipients             | IP ADDRESS         |
+                     +---------------------+-------------------------------+--------------------+
|                     | DOMAIN              | Name chosen by App developer  | PORT NUMBER        |
+                     +---------------------+-------------------------------+--------------------+
|                     | DATA                | Set by application            | DATA               |
+                     +---------------------+-------------------------------+--------------------+
|                     | TIMESTAMP           | Microseconds since 1970-01-01 |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | PATH                | Key to look up                |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | LINKS[]             | list of (Tag, Hash)           |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | PUBKEY & SIGNATURE  | Public key and Signature      |                    |
+---------------------+---------------------+-------------------------------+--------------------+
:::

[^4]: Both TCP/IP packets and linkspace packets have control fields that are irrelevant to a vast majority of developers.

All fields are optional except for HASH and DATA. For the full specs see the [guide](./docs/guide/index.html#packet_layout)

Linkspace is primarily this packet format. A minimal application only needs to broadcast/save packets.
The rest of the software library implements functions to process and query them.

### Merging Trees

One of the core ideas behind linkspace is a way to think about digital communication.
We'll imagine building a message forum.

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
A sorted list of multiple key-value pairs we'll call a **tree**.

The example shown has two trees **merging**. The result is a new tree with 3 messages.

Practically any digital communication can be talked about in terms of merging trees.

Online platforms have dubbed words for merging trees such as:
'_creating posts_', '_uploading image_', '_upvote/like a post_', '_stream a video_', etc.
Fundamentally they can be understood as a frontend application providing an interface to __merge__ trees.

The internet we use today has a single host design.
For instance, your browser or app contacts `http://www.some_platform.com`
for the key `/thread/BrokenMachine.jpg` to get the data.

The address `www.some_platform.com/thread/BrokenMachine.jpg` points to the only 'real' copy.

This is simple, but it has downsides.

A host can get disconnected,
copies can't be reshared and reused (thus once a host clears its storage the link is invalid),
and every host has their own strategy to deal with new data at the same path, i.e. an update.

I would argue they are all accidental complexity.
Especially the last one: Changing the value at a path.
Once the speed of light is measurable in a network, it is unavoidable for two computers to write to the same path without a costly synchronization steps.

In linkspace there is no such thing as a 'real' copy on a single host.

Anyone can read, write, and host (a partial) copy of the tree.
Every path refers to multiple values.

The values are distinct because each entry, i.e. **point** is cryptographically hashed.
i.e. there exists a -for all intents and purposes- unique 32 bytes ( or ~77-digit number) that identifies the entry (which i'll show as <span id="hh0" >[HASH_0]</span> instead of typing out).

Therefor it doesn't matter when or where trees are merged - and they only leave a single copy when both have the same message.

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
| /thread/Emacs or vim?/msg         | <span id="hh2">[HASH_2]</span> | I heard they're better then vs   |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Emacs or vim?/msg         | <span id="hh3">[HASH_3]</span> | Emacs with vim bindings ofcourse |
+-----------------------------------+--------------------------------+----------------------------------+
:::

<div class="op">=</div>
 
:::{.container .pkt .phd}
+-----------------------------------+--------------------------------+----------------------------------+
| /image/BrokenMachine.jpg          | <span id="hh0">[HASH_0]</span> | [image data]                     |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Emacs or vim?/msg         | <span id="hh2">[HASH_2]</span> | I heard they're better then vs   |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Emacs or vim?/msg         | <span id="hh3">[HASH_3]</span> | Emacs with vim bindings ofcourse |
+-----------------------------------+--------------------------------+----------------------------------+
| /thread/Coffee machine broke!/msg | <span id="hh1">[HASH_1]</span> | Fix pls? image/BrokenMachine.jpg |
+-----------------------------------+--------------------------------+----------------------------------+

:::

A point also has a creation date and can be signed - such that you can identify who created it.

As such, we can uniquely get a specific point by its <span id="hh0">[HASH_0]</span>,
or multiple entries through a path "/thread/Tabs or spaces/msg".

This might seem more trouble then existing solutions like a filesystem or HTTP.
However, in practice its trivial to emulate their behavior by adding constraints to the set; 
such as being the latest or the latest signed by a specific public key.
Conversely, I would argue both filesystems and HTTP are more trouble as both also return multiple values - a new value depending on when and where you make the query.

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

A point has two preceding fields. A **group** that signal who can read/write to the tree, and a **domain** field to indicate the application.
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

Note that for brevity I used the [LNS](./lns.html) representation for `[@:alice:salesexample]` and `[#:example]`.
In actually they are 32 bytes - similar to how `www.example.com` resolves to a number like `192.168.0.1`.

Finally, using the hash directly in the message like we have been doing is not ideal.
Instead a point in linkspace has a list of [links](./docs/guide/index.html#lk_linkpoint) adjacent to the data.

There are a more nuances and advanced topics such as: the path encoding, the [queries](./docs/guide/index.html#Query) syntax, and
specifics on how trees are [pulled](./docs/guide/index.html#lk_pull) from one server to another.
However, this should give you a good idea of the basics.

## Want to give it a try?

Check out [Webbit](https://github.com/AntonSol919/webbit) to see an application without installing anything.
You can [download](https://github.com/AntonSol919/linkspace/releases) a pre-build CLI and python library to follow along with
the [tutorial](./docs/tutorials/index.html) or the more technical [Guide](./docs/guide/index.html).

The linkspace packet format is stable and any packets you write will stay valid.
The library API can still break occasionally every now and then.

There are also a couple of parts not yet build:
The only group exchange process is just a bash script and its not easy to setup a private group exchange.

# Q&A

### Is linkspace a blockchain?

No.

Blockchains and supernets share a common idea:

Using cryptographic hashes and public keys to provide a model of data that trancends the connection used to share it.

Blockchain use the cryptography to batch multiple events into 'blocks'. As new blocks are made they are linked or 'chained' to the previous block.
This creates a consensus of all events that happened.

A blockchain's goal is to provide a model and tools to make consensus simple.

However, consensus on all events isn't that useful as a foundational feature for an overwhelming majority of applications.

Supernets don't bother with a global truth. They're goal is to work with the cryptographic links between packets.
Consequently, its not difficult to define a 'blockchain' style consensus in a general purpose supernet.

### Won't an app have a lot of overhead compared to a basic Web server? 

If all you want to do is stream one movie from a single host and forget it, then linkspace might be too much overhead.
Few projects stay that simple. Most projects grow in scope to identify users, save their comments, add them to groups, scale beyond a single server, etc.

Once a full stack is build, linkspace is very small w.r.t. its features.
Furthermore, it is designed to be fast/low energy, such that you can stream a video on a potato phone.

### Can you ask people to deal with the added complexity?

Yes.

Linkspace lack 3 decades of tooling that made the web relatively easy for users, but that can change. 

But is it worth it?

Yes.

Supernets better model the reality of multi party communication - asynchronous and authenticated[^auth]

[^auth]:Authenticated as in: cryptographicaly proven that messages were created by a user of a public key regardless where how you got the message - I call this the reality of multi party communication because a wire-dump of an HTTPS session is proof of autenticity of the host's message. Using that property is just unnecessarily complicated.

In the long run they can end up with less moving parts and with fewer configurations.

Most important above all, anyone can take responsibility.
It is dangerous to perpetuate a paradigm where users give away control and can't take it back.

My hope is to look back at this time as the era of digital fiefdoms. The next era - of the digital supernets - will hopefully balance out the influence of host-administrators. Users will define what a 'real' copy is; a digital space by people for people.

### Isn't it a good thing that a host can administrates what I and others see online?

There are a couple of options. You can trust the public key of third party service to whitelist content. Effectively emulating the current system.
But unlike the current system, you can replace them.
Furthermore, its trivial to vouch for your friends.

With AI driving the cost of bullshit to zero, such cross application identities will be critical.

### Won't it devolve to the same paradigm of centralized systems?

Maybe, maybe not. If a users can walk away from a host platform without much trouble, the host has to give a better deal than they do now.

## Why not [alternative]?{#alts}

There are two types I'm sure can't succeed at scale:

- Synchronizing chain of trust : Is slow and not useful for the vast majority of users.
- Faster email: (like ActivityPub) isn't a better abstraction (and also isn't that fast).

Other supernet-like systmes are limited in some way or simply made a different design decision:

- It has either hash addresses or custom url addresses, not both.
- Too slow. It should be doable in hardware and fast enough to stream video as is. Not hand it over to different protocol.
- No Groups. Consequently there is no or little granularity in what you share.
- No domains. Everything becomes one app.
- Its focused on signatures and consensus.
- A blessed/fixed method of exchanging data - either it fits your use-case or you're out of luck.
- Large 'packets' - a hash might refer to gigabytes. This requires multiple levels to deal with fragmentation in multiple ways.
- Poor (bash) scripting/piping support

That does not mean I think they are necessarily worse or useless.
Different systems have different strong points. More than one supernet can co-exist.
