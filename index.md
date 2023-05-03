<div class="definition">
Supernet  [ˈsü-pərˌnet]<br>
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.
A communication protocol where the method of exchange is an extraneous concern.<br>
e.g. git, bitcoin, nostr, linkspace
</div>

Linkspace combines the core ideas of HTTP and git.
It is a supernet. A protocol where we talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to control and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host which acts as the de facto administrator.

Linkspace is a supernet with the following highlights:

- Small API
- Fast packets (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressing
- Group/Domain split

[Basics](https://antonsol919.github.io/linkspace/index.html#basics) gives a high level introduction.
Check out the [tutorials](./docs/tutorial/index.html) to see an example of building an application.
For a technical description from first principles see the [Guide](./docs/guide/index.html).
[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)
to give it a try and say hi.

The packet and database layout are stable, but some things are incomplete or undocumented.

Any feedback, questions, and ideas for improvements are welcome!

Of course the preferred way is to send a message to the test group.
For the less adventurous you can open an issue on GitHub.

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
IO errors become the problem for the app instead of the people connected. There are also profound [consequences](#reason2) for the dynamic between host and user.

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
In an effort to put food on the table and pay taxes I prefer fiat money and a 1 cent transaction save a lot of trouble w.r.t. identification if you lose the private key.
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
