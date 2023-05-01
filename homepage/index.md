<div class="definition">
Supernet  [ˈsü-pərˌnet]<br>
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.
A communication protocol where the method of exchange is an extraneous concern.<br>
e.g. git, bitcoin, nostr, linkspace
</div>

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to own and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host which acts as the de facto administrator.

Linkspace is a supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable data
- Group/Domain split

[Basics](https://antonsol919.github.io/linkspace/index.html#basics) gives a high level introduction of the entire system.
Check out the [Guide](./docs/guide/index.html) if you're interested in the technical details.
[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try and say hi.

The packet and database layout are stable, but some things are incomplete or undocumented.

Any feedback, questions, and ideas for improvements are welcome!

Of course the preferred way is to send a message to the test group.
For the less adventurous you can open an issue on GitHub.

# Basics {#basics}

In the 1960s we invented the electronic hierachical filesystem.
Organizing files in folders. For example "/linkspace/homepage/index.html". This proved extremely powerful.
So much so, that the (early) Web, specifically HTTP, is essentially nothing more than a way to talk to file systems around the world.
i.e. "https://antonsol919.github.io/linkspace/index.html"
Linkspace takes this a step further.

To get an idea of what linkspace lets us do we can look at an example of a message board.
Initially it only contains someone complaing about a coffee machine and an attached image. Someone else starts a thread about tabs vs spaces.

The two are merged together to create the new state of the message board.

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
One or more of these entries, a path + data, in a (sorted) hierachical set together we'll call a **tree**. We saw what happens when we merge trees.
The internet as we know it is built on them.

There are millions of hosts (i.e. servers) that serve such a tree, receive new entries, and do some processing.

This is more true than might be aparent. It's not just HTTP.
For example, an SQL database is a special case of a tree. It is built on top of multiple sorted lists under table names.
Essentially their rows are "/table_name/primary_key = value". The SQL query can address and relate multiple entries.

The point is not to compare linkspace to HTTP or to replace SQL.
I bring them up as an argument for the universal effectiveness of organizing data in a such a tree.

Exchanging data can be thought of as combining **your tree** with **another tree**.
We've dubbed words to describe specific cases such as:
'__creating posts__', '__uploading image__', '__upvote/like a post__', '__stream a video __', etc.
Fundamentally they can be viewed as merging trees, with a frontend application providing a pretty interface.

The majority of the internet that people interact with today follows a single host design.
A design where you make a request to get the only 'real' copy of the tree.
For all its simplicity, this design has downsides.
It becomes a single point of failure, links can become invalid, everybody has to re-invent authentication, every application has to re-invent dealing with IO errors, etc. Additionally, there are profound [consequences](#reason2) for the dynamic between host and user.

In linkspace there is no single 'real' copy, and thus no de facto administrator.
Any number of participants can host (part of) a tree.

That does mean there is no way to uniquely identify an entry with only a path.
Two computers far apart can write to the same location at the same time.
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
These cryptographic public keys look like [b:0XITdhLAhfdIiWrdO8xYAJR1rplJCfM98fYb66WzN8c], but we can refer to them by [lns](#LNS) name such as [@:anton:nl].

An upside of using hashes, is that we can choose to link to other data by its path (e.g. "image/BrokenMachine.jpg") or by its hash:

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
Essentially each (domain, group) has its own tree.
An application picks a domain name to use. 
When running it specifies what data is required from the group.
The exchange of data happens in the background.
The application only has to deal with the tree and the new entries.
It doesn't have to manage connections.

The group indicates the set of intended recipients.
An application should ask the user which group to use.
A group is made up by members that have set up a method of exchange.


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
With [queries](./docs/guide/index.html#Query) we can read, filter, and request sets of packets.
Effectively hoisting the back-end and its administration into the control of the users and simplifying the life of a front end developer.

These are the basic concepts. With the most notable simplification being that: [Data entries](./docs/guide/index.html#lk_datapoint) without a path, group, domain, etc exists as well and referencing other packets by hash is not done inside the data but [adjacent](./docs/guide/index.html#lk_linkpoint) together with a 'tag'.

If you're on a unix give it a [try](https://github.com/AntonSol919/linkspace/releases) (It compiles on Windows, but an exchange like [anyhost](./docs/guide/index.html#anyhost) needs to be ported to rust to be usable).
For details on the exact layout of the tree and other practical stuff see the [Guide](./docs/guide/index.html).

### Q&A
A few notes to prevent some confusion.

**Q**: Is this a blockchain?  
**A**: Only if you think git is a blockchain. There are neither 'blocks', nor a strict 'chain'. Most blockchains also have a different set of [values](#option2). Which is why I'm proposing the more general term 'supernet'.

**Q**: How would it handle unwanted content / spam?  
**A**: You could trust a specific signature to whitelist or blacklist and filter based on that. Effectively emulating the current state of affairs. 
Furthermore, linkspace gives us extra tools: Proof of work on hashes, and proof of association with public keys vouching for another (i.e. friends of friends).

# Why?{#why}

This protocol came to be for two reasons.

Reason 1: I found it overly complex to build multi-user systems.

For every 'N' networked applications someone uses, they have to create 'M' groups.
This creates N*M configuration.
In theory linkspace makes this an N+M problem.

A group, once set up can run a domain application.
A domain application can run in any group.

Or put more practically:
The users can use a chat app to talk to a group and open a different app to play scrabble with that same group.

Similarly, from a developer's perspective, building something new can be fun.
Building something new and having to build and manage servers, user accounts, groups, etc is a lot less fun.

[Reason 2]{#reason2}: I wanted to have an alternative answer to a key question that underpins how our society uses digital systems.

**Who gets to __administrate__ the cat videos, taxi service, messages, and other data we share and see online?**

As I see it there are three options:

1. Applications are hosted by one organization on their machines. This is how most of the internet currently works.
2. A pay2play scheme, on a blockchain.
3. Users pick their admins, on a supernet.

In Option 1, our current web, the host has unilateral control as it relays data between users.
Furthermore, many popular hosts are the result of an environment where winner keeps all.
They're built for optimal exploitation of their users.
Everything is permitted to keep it that way.
**Lock-in** the users, and **lock-out** any threat to the platform's place in your life.

We should break these locks.

This dynamic is widespread in online system, not just in social media.
But to be clear, w.r.t. social media.
Collectively training an algorithm to keep you engaged is a personal choice. It is not the crux of the matter.
The problem is doing so in a paradigm where a few people have total and unshakable control over the experience of every user.

[Option 2]{#option2}, blockchains.
They are hyped to be many things, and some people believe blockchains will be the foundation of our digital space going forward.
I don't see how.
They encode scarcity, and using them is relatively expensive.
Very few systems require that.
Trusting an administrator is cheaper and works really well.

Scarcity makes blockchains attractive for the people already invested, but these properties are antithetical to the process of development.
Successful systems are build from small incremental improvements.
That dynamic doesn't seem to take place when the building blocks are a costly chain of consent.

Option 3, a supernet like linkspace, is the option that makes more sense to me.
It splits up the hosts and doesn't encode scarcity as a core principle.

In linkspace the role of host is split up between:

- providing groups with servers to relay data
- Developing domain applications that run on linkspace

It prevents both from having too much leverage and chasing the wrong incentives.
Should someone in the system abuses their position the user can change things up without losing access to their history and relations to others.
If for nothing else, a supernet is worth it to get us more useful competition.

Whether this all works out as intended, I have no idea.

My hope is we can look back at the current era of the internet and recognize it for what it is.
digital fiefdoms.
The step forward is to take full control over who lords over us.

Help speed things along.
You can support the project by registering a public LNS name.

# LNS{#LNS}

LNS is built on top of linkspace.  
It uses the lns domain to be exact.  

Groups and public keys are 32 bytes. Unreadable for humans.
LNS enables us to assign them names.
Both publicly by registering, and privately for your own convenience.  

**Groups** look like:  

- \#:pub
- \#:myfancystore:com
- \#:friendsofbob:local

**Public keys** look like:

- @:john:lns:org
- @:alice:my:fancystore:com
- @:john:nl
- @:me:local

The top level names :local, :env, and :pub are special.
You can take a look on how it integrates with linkspace in the [guide](./docs/guide/index.html#ABELNS).

LNS is currently only partially operational.

Registrations for public names are open though.
Read this page to find out how.

### What does a registration do?

It gives you binding right for all sub registration.
That means if you register under @:yourcompany:com you can set up:

1) a key addressed with the name @:yourcompany:com
1) a group addressed with the name #:yourcompany:com
1) a key to manage registrations for names ending with *:yourcompany:com

I.e. The key with the authority for yourcompany:com can create bindings for sales:yourcompany:com.
That registration can set up a binding for the key @:bob:sales:yourcompany:com.

### Do I require a registration?

Nothing in linkspace requires a registration.
Everything can be done without.
In fact, \*:env names you set up for your own use (similarly to /etc/hosts), and \*:local are meant for names you share between peers.
Registrations allow you to pick an unambiguous name to be known publicly, and they support the project financially.

### How can I register?

The following top level authorities have been assigned, and you can request a name ending in:

- :free. First come, first served.
- :dev if you have contributed to the code.
- :com for 10 euro per year.
- :nl for 10 euro per year.

To do so, see [claim a name](#claim).

### Can I become a top level authority?

Yes. I am looking for people and organizations to do so.
Contact me at <antonsol919+registar@gmail.com> for more information.
If you represent a university you can get your name for free.

### Can I buy with crypto? Why not use crypto to do X, Y, Z ?

Linkspace is easy to integrate with blockchains (or can be used to create new blockchains).
You are free to build on it as you wish (MPL-2.0 license).
However, in an effort to put food on the table and pay taxes I prefer fiat money and a 1 cent transaction save a lot of trouble w.r.t. identification if you lose the private key.
Other top level authorities set their own price and how to pay it.

## Claim a name{#claim}

This currently requires a some work.
To make a :free claim get download or clone the repo.

```terminal
source ./activate
lk --init key --key 'YOUR_NAME:local' | tee enckey
lk lns create-claim 'YOUR_NAME:free' --copy-from YOUR_NAME:local --until [now:+99Y] | tee lnsreq.lkp | lk p
```

Keep the file 'enckey' (and the password you entered) safe.
Email the lnsreq.lkp file to <antonsol919+lns@gmail.com>.
I plan to accept :free name requests until it becomes a burden,
 after which I'll probably put up a proof of work fence with some additional constraints on the name.

Get a pull request accepted, and you get a :dev name.

First come, first served.

# About {#about}

Linkspace is currently an unfunded project, and is missing some key [features](https://github.com/AntonSol919/linkspace/blob/main/dev/TODO.md).
Meaning I do other stuff to make a living and won't be available all the time.

You can contact me directly at <AntonSol919@gmail.com> if you're interested in supporting the project.
Or if you want to talk about specific applications.

### Trademark

linkspace is a pending trademark of R.A.I.T. Solutions.
Once registered I will likely use similar terms as git does for its trademark.
