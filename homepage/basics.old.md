# Basics {#basics}

In the 1960s we build the first electronic hierachical filesystems.
It took a while, but eventually the idea to organize in 'files and folders' became common.
For example "documents/linkspace/homepage/readme.md".

The (early) Web, specifically HTTP, made these folders available and allowed for cross linking.
i.e. "https://antonsol919.github.io/linkspace/index.html"
Linkspace takes this a step further.

To build an understanding of linkspace we'll begin with a basic message board.
Someone complaining about a coffee machine, and some else starts a thread about tabs vs spaces.

The entries get merged together.

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

We'll call "image/BrokenMachine.jpg" a **path** pointing to [image data].
The hierarchical (sorted) set of path + data we'll call a **tree**.

There are millions of hosts (i.e. servers) that conceptually serve such a tree, merge new entries, and do some processing.

This is more true than might be aparent. It's not just HTTP.
For example, an SQL database stores its data in a tree.
Essentially their rows are "/table_name/primary_key = value".
The SQL query can address and relate multiple entries.

Trees are our best tools for organizing data. Both human and machine readable.

Exchanging data can be thought of as combining **your tree** with **another tree**.
We've dubbed words to describe specific cases such as:
'__creating posts__', '__uploading image__', '__upvote/like a post__', '__stream a video __', etc.
Fundamentally its a frontend application providing an interface to merge trees.

The majority of the internet that people interact with today follows a single host design.
A design where you request to get the only 'real' copy of the tree.
This is simple, but has downsides.
It becomes a single point of failure, links become invalid, every new app requires developers and users to build and manage accounts and connections,
IO issues get blamed on the app developer instead of being the problem of the people communicating. There are also profound [consequences](#reason2) for the dynamic between host and user.

In linkspace there is no 'real' copy.
Anyone can read, write, and host (part of) a tree.

This does mean we must deal with two entries using the same path.
Two computers far apart could write to the same location at the same time.
No one would know until their trees get merged.

In linkspace we allow more than one entry to have the same path.
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
These cryptographic public keys look like [b:0XITdhLAhfdIiWrdO8xYAJR1rplJCfM98fYb66WzN8c]. We can refer to them by an [lns](#LNS) name such as [@:anton:nl].

Because we have a hash, we can choose how to reference data.

A specific entry by its <span id="hh2">[HASH_2]</span>,
or multiple entries through a path "/thread/Tabs or spaces/msg".

On first glance, returning multiple entries is more complex than what is familiar.
This is not the case.
Instead it makes explicit what is implicit.
URLs also return more than one unique results depending on when you look.

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
The user running a linkspace instance has to run a group exchange process and configure its members.

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

We can be more specific in our references than just using a path with [queries](./docs/guide/index.html#Query).
This is a single syntax to read, filter, and request sets of packets.

These are the basic concepts. With the most notable simplification being that: 
Referencing other packets by hash is not done inside the data but [adjacent](./docs/guide/index.html#lk_linkpoint) together with a 'tag'.
[Data entries](./docs/guide/index.html#lk_datapoint) without a path, group, domain, etc exists.

If you're on a unix give it a [try](https://github.com/AntonSol919/linkspace/releases) (It runs on Windows, but there is currently no working exchange process).
Check out the application [tutorial](./docs/tutorial) tutorial to see what applications in action.
For the full overview of linkspace see the [Guide](./docs/guide/index.html).


### Q&A
A few notes to prevent some confusion.

**Q**: Is this a blockchain?  
**A**: No more than git is. No blocks, no chains, and no money/electricity/stakes required for developers and users to use it.

**Q**: How would it handle unwanted content / spam?  
**A**: Just as we do now. With the additional tools of: hashes, digital signatures, and proof of work.
