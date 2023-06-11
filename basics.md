# Basics

## The internet of streams

The internet attempts to provide a model where: for any two points running any application, there exists a connection to transmit data.

To do this we use the following types of packets.

:::{.container}
+-----------------+-----------------------+-----------------------------------------------------------------+
|                 | Field                 | Meaning                                                         |
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

Packets are transmitted using some piece of hardware, and eventually reach a destination `IP address`, such as a phone.
At the destination an application is listening for packets with a specific `port` to be received.

Packet don't arrive in the order they were sent, the speed of light makes this impractical.
By adding a sequence number, we can reorder them at the destination.

The result is that _conceptually_ each application on each device can talk to any other application on any other device.

This model is ideal for phone-calls or video streams.
To build more interesting applications we encode our data in a specific way.
There are thousands of different encodings, but what they share is that they transmit questions and answers.
Or in other words: queries and responses, or keys and values[^jargon].
i.e. a mapping between input and output.

[^jargon]: In some situations there is a technical difference between "query-response" and "key-value" systems.
But when describing them in a network (where origin and time are implied) they are indistinguishable.

Over the years we've built a thousand different protocols to facilitate this design.
Some are extremely specific to a use-case, some are more generic.

A couple of well known systems that can map keys to values over the internet are:

| System | Query                         | Value               |
|--------+-------------------------------+---------------------|
| DNS    | archive.org                   | 207.241.224.2       |
| HTTP   | /forum/index.html             | Hello world!        |
| FTP    | /Projects/linkspace/readme.md | In a supernet [...] |
| SQL    | SELECT * from MSG where ID=1; | A message in a db   |

The reason for building linkspace is this:

**We have reached the limit of this paradigm.**

## Linkspace

Linkspace attempts to provide a model where: for any group running any application, there exists a space to address data.

If the current internet provides streams for key-value systems, so we can talk _to_ server,  
then linkspace provides a shared key-value space, so we can talk _about_ data.

A unit in linkspace is called a point. Each point has data, some auxiliary fields, and is uniquely identified by its hash.

:::{.container}
+---------------------+---------------------+-------------------------------+--------------------+
|                     | Field               | Meaning                       | IP Packet Analogue |
+=====================+=====================+===============================+====================+
| Linkspace Point[^4] | HASH                | A unique ID                   |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | GROUP ID            | Set of recipients             | IP ADDRESS         |
+                     +---------------------+-------------------------------+--------------------+
|                     | DOMAIN              | Name chosen by App developer  | PORT NUMBER        |
+                     +---------------------+-------------------------------+--------------------+
|                     | DATA                | Set by application            | DATA               |
+                     +---------------------+-------------------------------+--------------------+
|                     | TIMESTAMP           | Microseconds since 1970-01-01 |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | PATH                | Key with upto 8 components    |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | LINKS[]             | list of (Tag, Hash)           |                    |
+                     +---------------------+-------------------------------+--------------------+
|                     | PUBKEY & SIGNATURE  | Public key and Signature      |                    |
+---------------------+---------------------+-------------------------------+--------------------+
:::
 
   
[^4]: Both IP packets and linkspace packets have control fields that are irrelevant to a vast majority of developers. The key word being 'attempt' to provide a model. 

Fields are optional[^5] except for the hash. i.e. not every packet has to be signed.

[^5]: Optional is slightly misleading. There are 3 types: datapoint, linkpoint, and keypoint.  For a full specification checkout the guide.

### Merging Trees

To better understand what each field does, imagine a message platform build on a basic key-values system similar to a file system.

<div class="entrygrid small"><span></span>
<span>/image/BrokenMachine.jpg</span>
<span>[image data]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg</span>
<span>fix pls? image/BrokenMachine.jpg</span>
</div>

<div class="op">+</div>

<div class="entrygrid small">
<span></span>
<span style="width:28ch">/thread/Tabs or spaces/msg</span>
<span>Are we still doing this?</span>
</div>

<div class="op">=</div>

<div class="entrygrid small">
<span></span>
<span>/image/BrokenMachine.jpg</span>
<span>[image data]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg</span>
<span>fix pls? image/BrokenMachine.jpg</span>
<span></span>
<span>/thread/Tabs or spaces/msg</span>
<span>Are we still doing this?</span>
</div>

The "image/BrokenMachine.jpg" is a **path** pointing to [image data].
The hierarchical (sorted) set of path + data we'll call a **tree**.

The example has two trees merging .
Merging trees is a powerful abstraction.

Take a website or phone app. 
We've dubbed words to describe their specific features such as:
'_creating posts_', '_uploading image_', '_upvote/like a post_', '_stream a video_', etc.
Fundamentally they can be understood as a frontend application providing an interface to __merge__ trees.

The internet we use today has a single host design.
A design where applications make request to a specific address in order to get the only 'real' copy of the tree.

This is simple, but having only one real copy has downsides.
It becomes a single point of failure, links become invalid, a copy can't be reshared and reused, and other limits we'll come back to.

In linkspace there is no 'real' copy.
Anyone can read, write, and host (part of) a tree.

This does mean we must deal with two entries using the same path.
Two computers far apart could write to the same location at the same time.
No one would know until their trees get merged.

In linkspace the entries, i.e. **points**, can share the same path.
Each point is cryptograhpically hashed.
This means there exists a unique number to reference it.

<div class="entrygrid small"><span id="hh0">[HASH_0]</span><span>/thread/Tabs or spaces/msg</span><span>Are we still doing this?</span></div>

<div class="op">+</div>

<div class="entrygrid small">
<span id="hh1">[HASH_1]</span>
<span>/thread/Tabs or spaces/msg</span>
<span>Why not U+3164?</span>
</div>

<div class="op">+</div>

<div class="entrygrid small">
<span id="hh2">[HASH_2]</span>
<span>/thread/Tabs or spaces/msg</span>
<span>Get a life</span>
</div>

<div class="op">=</div>

<div class="entrygrid small">
<span id="hh0">[HASH_0]</span>
<span>/thread/Tabs or spaces/msg</span>
<span>Are we still doing this?</span>
<span id="hh1">[HASH_1]</span>
<span>/thread/Tabs or spaces/msg</span>
<span>Why not U+3164?</span>
<span id="hh2">[HASH_2]</span>
<span>/thread/Tabs or spaces/msg</span>
<span>Get a life</span>
</div>

A point can also carry a creation date, and can be cryptographically signed.
These cryptographic public keys look like [b:0XITdhLAhfdIiWrdO8xYAJR1rplJCfM98fYb66WzN8c].
Eventually we'll be able to refer to them by a [lns](./lns.html) name such as [@:anton:nl].

Because we have a hash and a path, we can choose how to reference data.

We can reference a specific point by its <span id="hh2">[HASH_2]</span>,
or multiple entries through a path "/thread/Tabs or spaces/msg".

At first glance, returning multiple entries for a custom key is more complex than the familiar filesystem or HTTP.
This is not entirely true.
Instead, they have implicitly multiple values per key.
It depends on when a request is made, or even from where.

As such, a message board could look like.

<div class="entrygrid big">
<span id="hh3">[HASH_3]</span>
<span></span>
<span>/image/BrokenMachine.jpg<br>2015/01/02</span>
<span>[image data]<br>[@:alice:sales:com]</span>
<span id="hh4">[HASH_4]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/02</span>
<span>fix pls? <span id="hh3">[HASH_3]</span><br>[@:alice:sales:com]</span>
</div>

<div class="op">+</div>

<div class="entrygrid big">

<span id="hh5">[HASH_5]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/03</span>
<span>
Hey <span id="hh4">[HASH_4]</span>!
Isn't this <span id="hh3">[HASH_3]</span> image from 2015?<br>[@:bob:maintainance:com]
</span>
</div>

<div class="op">=</div>

<div class="entrygrid big">
<span id="hh3">[HASH_3]</span>
<span></span>
<span>/image/BrokenMachine.jpg<br>2015/01/02</span>
<span>[image data]<br>[@:alice:sales:com]</span>
<span id="hh4">[HASH_4]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/02</span>
<span>fix pls? <span id="hh3">[HASH_3]</span><br>[@:alice:sales:com]</span>
<span id="hh5">[HASH_5]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/03</span>
<span>
Hey <span id="hh4">[HASH_4]</span>!
Isn't this <span id="hh3">[HASH_3]</span> image from 2015?
<br>
[@:bob:maintainance:com]
</span>
</div>

We do want to control who access what, how, and when.

To do so, points in linkspace have two fields that precede the path.
A **domain** field and **group** field.
Essentially each combination of (domain, group) has its own tree.

The group indicates the set of intended recipients.
The device running linkspace instance run and configure a group exchange process to merge trees with others.

An application picks a domain name.
It only has to interfaces with the tree.
Not with managing connections.

<div class="entrygrid big">
<span id="hh3">[HASH_3]</span>
<span>message_board<br>[#:example:com]</span>
<span>/image/BrokenMachine.jpg<br>2015/01/02</span>
<span>[image data]<br>[@:alice:sales:com]</span>
<span id="hh4">[HASH_4]</span>
<span>message_board<br>[#:example:com]</span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/02</span>
<span>fix pls? <span id="hh3">[HASH_3]</span><br>[@:alice:sales:com]</span>
<span id="hh5">[HASH_5]</span>
<span>message_board  [#:example:com]</span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/03</span>
<span>
Hey <span id="hh4">[HASH_4]</span>!
Isn't this <span id="hh3">[HASH_3]</span> image from 2015?<br>[@:bob:maintainance:com]
</span>
</div>

Finally, using the hash directly in the content of the data is not ideal for many reasons. 
Few data formats have a notion of references, and more importantly they're difficult for machines to read.
Instead a point in linkspace has a list of [links](./docs/guide/index.html#lk_linkpoint) adjacent to the data.

There are a couple more advanced topics, such as how paths are encoded the [queries](./docs/guide/index.html#Query) system. 
But this covers the basic idea behind linkspace;

Applications reading, writing, and reacting to points as they query their domain tree,
users joining groups by configuring how they exchange points.

## Ready to try?

The linkspace library has a mostly stable API.
With the CLI you can quickly script a bridge between streams and linkspace, or build a new application.

However, this is still a work in progress.

If you're on a unix give it a [try](https://github.com/AntonSol919/linkspace/releases) and say hi on the test group,
emulate a local group, or start building your own.

(It runs on Windows, but there is currently no working group exchange process.)

For a technical document regarding the API and CLI see the [Guide](./docs/guide/index.html).
If you want to support the project consider registering a [public name](./lns.html).

# Q&A

Some common questions and answers about the project in general:

### Is linkspace a blockchain?

No.

Blockchains and supernets share a common dea:

Using cryptographic hashes and public keys to provide a model of data that trancends the connection used to share it.

Blockchain use the cryptography to batch multiple events into 'blocks'. As new blocks are made they are linked or 'chained' to the previous block.
This creates a consensus of all events that happened.

A blockchain's goal is to provide a model and tools to make consensus simple.

However, consensus on all events isn't that useful as a foundational feature for an overwhelming majority of applications.

A supernet simply does not require that kind of consensus.

With a general purpose supernet like linkspace it is easy to implement
similar consensus mechanism in an application.

### Won't a app have a lot of overhead compared to a basic Web server? 

If all you want to do is stream one movie from a single host and forget it, then linkspace might be too much overhead.
Few projects stay that simple. Most projects grow in scope to identify users, save their comments, add them to groups, scale beyond a single server, etc.

Once a full stack is build, linkspace is very small w.r.t. its features.
Furthermore, it is designed to be fast/low energy, such that you can stream a video on a potato phone.
Even single threaded: `dd bs=10G count=1 if=/dev/zero | lk data > /dev/null`

### Can you ask people to deal with the added complexity?

Yes.

Linkspace lack 3 decades of tooling that made the web relatively easy for users, but that can change.

The nature of communication over distance is chaotic and asynchronous, so much of the "complexity" is not artificial or accidental.
Furthermore, the number of configuration could end up smaller: Passwords, Groups, friends can be setup once and used by every application.
Finally, making people responsible isn't a bad thing, being impotent online is.

### Isn't it a good thing that a host administrates what I and others can see online?:

This can be a service that is provided so end users do not have to worry about seeing questionable content.
Similar to how it is done today.

On the flip side, hosts are currently final and total administrators by virtue of hosting the data.
Having this as the foundational 'truth' to how billions of people spend hours each day communicating scares the shit out of me.

### Won't it devolve to the same paradigm of centralized systems?:

Maybe, maybe not. If a users can walk away from a host platform without losing their history, the host has to give a better deal than they do now.

## Why not <alternative>?

Some limitations i've found are:

- It has either hash addresses or custom url addresses, not both.
- Too slow. It should be doable in hardware and fast enough to stream video as is. Not hand it over to different protocol.
- No Groups. Consequently there is no or little granularity in what you share.
- No domains. Everything becomes one app.
- Its distracted with facilitating digital signatures and concensus, instead of focusing on the utility without a consensus.
- A blessed/fixed method of exchanging data, instead of a external/modular system to be filled in per use case.
- An ever growing set of built in (stream) protocols to negotiate a state. (A consequence of the previous point)
- A stagnating set of protocols and thus improvements are hard to roll out.
- Poor (bash) scripting/piping support

That does not mean I think alternative are worse or useless.
Different models have different strong points, more than one supernet can co-exist.
