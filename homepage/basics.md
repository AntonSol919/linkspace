# Basics

## The TCP Internet

The TCP internet attempts to provide a model where: for any two points running any application, there exists a connection to transmit data.

It does this by using the following types of packets:

| Internet Packet |
|-----------------|
| IP ADDRESS      |
| SEQUENCE ID     |
| PORT            |
| DATA            |

Packets are transmitted, and eventually reach a destination `IP address`, such as your phone.
At the destination an application is listening for packets with a specific `port` to be received.

Packet don't arrive in the order they were sent, the speed of light makes this impractical.
By adding a sequence number, we can reorder them at the destination.

The result is that _conceptually_ each device has a direct connection to every other device.
An application can send and receive streams of data to and from others.

This is ideal for phone-calls or video streams.
To create more dynamic applications we build mappings between 'keys' and 'values'.
Using a query we can select one or more keys and their value.

For example:

| System | Query                         | Value               |
|--------|-------------------------------|---------------------|
| DNS    | archive.org                   | 207.241.224.2       |
| HTTP   | /forum/index.html             | Hello world!        |
| FTP    | /Projects/linkspace/readme.md | In a supernet [...] |
| SQL    | SELECT * from MSG where ID=1; | A message in a db   |

By transmitting the query and result values using these streams we build the web as we know it today.

The reason for building linkspace is this:

**We have reached the limit of this paradigm.**

To under why, let's first explain linkspace.

## Linkspace

Linkspace attempts to provide a model where: for any group running any application, there exists a space to address data.

If the TCP internet provides streams for key-value systems, so we can talk _to_ server,  
then linkspace provides a shared key-value space, so we can talk _about_ data.

A unit in linkspace is called a point. Each point has data, some auxiliary fields, and is uniquely identified by its hash.

| Linkspace Point[^4] | Notes                         | IP Packet Analogue |
|---------------------|-------------------------------|--------------------|
| HASH                | A unique ID                   |                    |
| GROUP ID            | Set of recipients             | IP ADDRESS         |
| DOMAIN              | Name chosen by App developer  | PORT NUMBER        |
| DATA                |                               | DATA               |
| TIMESTAMP           | Microseconds since 1970-01-01 |                    |
| PATH                | Key with upto 8 components    |                    |
| LINKS[]             | list of (Tag, Hash)           |                    |
| PUBKEY & SIGNATURE  | Public key and Signature      |                    |
    
[^4]: Both IP packets and linkspace packets have control fields that are irrelevant to a vast majority of developers. The key word being 'attempt' to provide a model. 

The auxiliary fields are optional[^5], e.g. a point does not have to be signed.

[^5]: Optional is slightly misleading. There exist 3 types: datapoint, linkpoint, and keypoint.  For a full specification checkout the guide. 

### Merging Trees

To understand the fields and linkspace overall, imagine a message platform build on a basic key values system similar to a file system.

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

The example has two entries merging. 
Merging trees is a powerful abstraction.
It is essentially how websites/apps work.

Sending data can be understood as merging your tree into another tree.

We've dubbed words to describe specific cases such as:
'_creating posts_', '_uploading image_', '_upvote/like a post_', '_stream a video_', etc.
Fundamentally they are the frontend application providing an interface to __merge__ trees.

The internet we know has a single host design.
A design where you request to get the only 'real' copy of the tree.

This is simple, but having only one real copy has downsides.
It becomes a single point of failure, links become invalid, you cannot reuse or share your copy, and other limits we'll come back to.

In linkspace there is no 'real' copy.
Anyone can read, write, and host (part of) a tree.

This does mean we must deal with two entries using the same path.
Two computers far apart could write to the same location at the same time.
No one would know until their trees get merged.

In linkspace entries can share the same path.
Each entry is cryptograhpically hashed, i.e. there exists a unique number to reference the entry.

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

An entry also carries a creation date, and can be cryptographically signed.
These cryptographic public keys look like [b:0XITdhLAhfdIiWrdO8xYAJR1rplJCfM98fYb66WzN8c]. We can refer to them by a [lns](#LNS) name such as [@:anton:nl].

Because we have a hash, we can choose how to reference data.

We can reference a specific entry by its <span id="hh2">[HASH_2]</span>,
or multiple entries through a path "/thread/Tabs or spaces/msg".

At first glance, returning multiple entries is more complex than what is familiar.
This is not entirely true.
Instead, it makes explicit what is implicit.
URLs also return more than one unique results, it just depends on when you look.

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

Entries in linkspace have two fields that precede the path.
A **domain** field and **group** field.
Essentially each combination of (domain, group) has its own tree.

An application picks a domain name.
When running it signals what data is required from the group.
A developer deals with reading and writing the tree.
Not with managing connections.

The group indicates the set of intended recipients.
The device running linkspace instance run and configure a group exchange process to exchange data with others.

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

We can be more specific than just using a path with [queries](./docs/guide/index.html#Query).

This is the basic idea of linkspace. Users merging trees with groups so that applications can read, write, and react to data in their domain.

The most notable simplification being that referencing other packets by hash is not done directly inside the data but in the [link](./docs/guide/index.html#lk_linkpoint) field of each point.

## The promise of linkspace

With a basic understanding we can compare linkspace with streams of key values, and show the limit of the latter

Conceptually linkspace allows users and applications to skip past the network of streams, and instead reason about a shared space.

For developers,
an application like the [example](./docs/tutorial/mineweeper/01.html) that is equivalently: scalable, always-online, extendable system would be a large undertaking difficult to justify.
While paying to run the dozen services required to provide the users with similar features, you'll have to do it all again for a next project.

The other limit is our collective perception.

The host provides an app or website to talk to them.
This interface provides the concepts with which people understand the internet.

In the last two decades this relationship has ingrained a set of beliefs:

- Tools/apps only work within a single ecosystem (corollary - external tools can/must have permission of the platform to work)
- Users should submit to companies that provide illiquid value on the condition of their loyalty.

Both are misconceptions not grounded in the reality of what is possible.

With some know-how you can process any data in any way, but the host can choose to make it difficult.
Because it requires this know-how, people tend to accept the story told by the platforms.

We're all worse off for it.

Finally, there is a long list of issues others have identified on the platforms they use, such as:
privacy, addiction, radicalization, anti-consumer monopolies, excessive spam/advertisement, a glut of AI generated content, etc.

If we limit our self to the internet as it is now these issues are daunting, and come with vague or impractical solutions.
Viewed from a broader perspective, these issues are a consequence of [how we choose to administrate](./why.html#reason2).

## Ready to try?

The linkspace library has a stable API.
With the CLI you can quickly script a bridge between streams and linkspace, or build a new application.
However, there are still rough edges and some missing pieces to make the user experience easy.

If you're on a unix give it a [try](https://github.com/AntonSol919/linkspace/releases) and say hi on the test group,
emulate a local group, or start building your own.

(It runs on Windows, but there is currently no working group exchange process.)

For a technical document regarding the API and CLI see the [Guide](./docs/guide/index.html).
If you want to support the project consider registering a [public name](./lns.html).
