```definition
Supernet  [ˈsü-pərˌnet]
A self-referential multi-participant data organization protocol whose primary
addressing method uses hashes instead of endpoint identifiers.
A communication protocol where the method of exchange is an extraneous concern.
e.g. git, bitcoin, nostr, linkspace
```

In a supernet anybody can talk _about_ data, instead of talking _at_ a server.

A supernet is ideal when multiple participants want to own and administrate (part of) a digital system.
This is in contrast to current technologies where users contact a single host,
which acts as de facto administrator by virtue of hosting the data.

Linkspace is a supernet with the following highlights:

- Small and powerful API
- Fast (Blake3, no JSON/Base64 encoding, well aligned fields)
- Path (URL like) addressable data
- Group/Domain split

Check out the [Basics](#basics) for an introduction.
[Download](#download) to give it a try and say hi on the test group.
Check out the [Guide](./docs/guide/index.html) if you're up for some programming.

The packet format and index are stable, but expect some unimplemented features and rough edges.

# Basics {#basics}


In the 1960s we invented the electronic hierachical filesystem. 
Organizing files in folders. For example "/linkspace/homepage/index.html". This proved extremely powerful.
So much so, that the Web (specifically HTTP) is essentially nothing more than a way to talk to file systems around the world.
i.e. ["https://antonsol919.github.io/linkspace/index.html"](http://antonsol919.github.io/linkspace/index.html).
Linkspace takes this a step further. 

To understand linkspace let's look at an example of a message board.
Initially it only contains someone complaing about a coffee machine. Someone else starts a thread about tabs vs spaces. 

The two are merged together to create the new state of the message board.

<div class="entrygrid small"><span></span>
<span>/image/Broken.jpg</span>
<span>[image data]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg</span>
<span>fix pls? image/Broken.jpg</span>
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
<span>/image/Broken.jpg</span>
<span>[image data]</span>

<span></span>
<span>/thread/Coffee machine broke!/msg</span>
<span>fix pls? image/Broken.jpg</span>

<span></span>
<span>/thread/Tabs or spaces/msg</span>
<span>Are we still doing this?</span>

</div>

We'll call "image/Broken.jpg" a **path** pointing to [image data].
One or more of these entries, a path + data, in a (sorted) hierachical set together we'll call a **tree**. We just saw what happens when we merge trees.
The internet as we know it is built on them.
There are millions of hosts (i.e. servers) that serve such a tree, receive new entries, and do some processing.

This is more true than might be aparent. It's not __just__ HTTP. 
For example, an SQL database is a special case of a tree. It is built on top of multiple sorted lists under table names.
    Essentially their rows are "/table_name/primary_key = value", and a SQL query can address multiple entries.

Sending data can be thought of as combining **your tree** with another **tree**.
We've dubbed words to describe specific cases such as:
'__creating posts__', '__uploading image__', '__upvote/like a post__', '__stream a video __', etc.
Fundamentally they can be seen as merging trees.

The majority of the internet that people interact with today follows a single host design.
A design where you make a request to get the only 'real' copy of the tree.
For all its simplicity, this design has techinical downsides[^1]
and comes with profound [consequences](#why) for the dynamic between host and user.

In linkspace there is no single 'real' copy, and thus no de facto administrator.
Any number of participants can host (part of) tree.

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


An entry also carries a creation date, and __can__ be cryptographically signed. 
These cryptographic public keys look like [b:0XITdhLAhfdIiWrdO8xYAJR1rplJCfM98fYb66WzN8c], but we can refer to them by [lns](#LNS)  name such as [@:anton:nl].

When reading from the tree, requesting by path returns multiple entries.
By default, the first result is the latest, unsigned entry.

An upside of using hashes, is that we can choose to link to other data by its path (e.g. "image/Broken.jpg") or by its hash:

<div class="entrygrid big">

<span id="hh3">[HASH_3]</span>
<span></span>
<span>/image/Broken.jpg<br>2015/01/02</span>
<span>[image data]<br>[@:alice:sales:com]</span>

<span id="hh4">[HASH_4]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/02</span>
<span>fix pls? image/Broken.jpg<br>[@:alice:sales:com]</span>
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
<span>/image/Broken.jpg<br>2015/01/02</span>
<span>[image data]<br>[@:alice:sales:com]</span>

<span id="hh4">[HASH_4]</span>
<span></span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/02</span>
<span>fix pls? image/Broken.jpg<br>[@:alice:sales:com]</span>

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

Entries in linkspace have two fields that preceed the path.
A **domain** field and **group** field.
Essentially each (domain, group) has its own tree.
A developer building an application can pick a domain name, and builds his app to read and write the data structure he needs for his application as entries in the tree.
A domain app doesn't need to manage connections to other servers. It communicates by reading and writing to the tree.
The group indicates the set of intended recipients.
An application should ask the user which group to use.
A group is made up by one or more members that have set up a method of exchange.
    
<div class="entrygrid big">

<span id="hh3">[HASH_3]</span>
<span>message_board<br>[#:example:com]</span>
<span>/image/Broken.jpg<br>2015/01/02</span>
<span>[image data]<br>[@:alice:sales:com]</span>

<span id="hh4">[HASH_4]</span>
<span>message_board<br>[#:example:com]</span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/02</span>
<span>fix pls? image/Broken.jpg<br>[@:alice:sales:com]</span>

<span id="hh5">[HASH_5]</span>
<span>message_board  [#:example:com]</span>
<span>/thread/Coffee machine broke!/msg<br>2023/03/03</span>
<span>
Hey <span id="hh4">[HASH_4]</span>!
Isn't this <span id="hh3">[HASH_3]</span> image from 2015?<br>[@:bob:maintainance:com]
</span>

</div>

These are the basic concepts[^2].
If you're on a unix give it a [try](#Download).
For details on the exact layout of the tree and other practical stuff see the [Guide]("./docs/guide/index.html").

### Q&A
A few notes to prevent some confusion.

**Q**: Is this a blockchain?  
**A**: Only if you think git is a blockchain. There is no strict 'chain', nor 'blocks'. I consider blockchains to have a different set of [values](#option2). Which is why I'm proposing 'supernets'.

**Q**: Isn't administration necesary for such and such?  
**A**: Its relativly simple to organize such that a participant trusts a specific key to act as administrator. The difference sits in having a choice or not.  

**Q**: Will it handle spam?  
**A**: No worse than current systems. Instead, we have extra tools. Proof of work on hashes, and proof of association with public keys. With AI advances driving the cost of bullshit to 0, they'll become a necesity. Will it solve it entirely? A philosphical discussion on the classification of spam is outside the scope of this project.

[^1]: They're a single point of failure, links can become invalid, everybody has to re-invent authentication, everybody has to re-invent dealing with IO errors, scaling requires techinical know-how, etc.
[^2]: With the most notable simplification being that: [Data entries](./docs/guide/index.html#lk_datapoint) without a path, group, domain, etc exists as well. Referencing other packets by hash is not done inside the data but [adjacent](./docs/guide/index.html#lk_linkpoint).

# Why?{#why}

This protocol came to be for two reasons.

Reason 1: I found it overly complex to build multi-participant systems.

A project starts and ends with a vision for what the user should experience and do.
That's difficult enough.
Instead, development also has to deal with managing a server, networking, identity, and access.
In linkspace I've tried to decouple these things.
A group, once setup can run any domain app. A domain app can run in any group.

Reason 2: I wanted to address a key question that underpins how our society uses digital systems.

**Who gets to __administrate__ the cat videos, taxi service, messages, and other data we share and see online?**

I know three options:

1. Dedicated hosts, exclusively on their machines. This is how most of the internet currently works.
2. A pay2play scheme, on a blockchain.
3. Users pick their admins, on a supernet.


In Option 1, our current web, control over content is unilateral.
The current systems have evolved with one goal:
optimal exploitation of their users. Everything is permitted to keep it that way.
**lock-in** the users, and **lock-out** any threat to the platform's place in your life.

We should break these locks.

Dedicated hosts have a role to play.
But users can only get a good deal if they _could_ walk away without losing what is already there.

[Option 2]{#option2}
, blockchains. They are hyped to be many thinks, and some people want to believe blockchains will be the foundation of our digital space going forward. I don't see how. They encodes exclusivity, scarcity, and inequality.
Very few useful systems require those things.
It makes them attractive for the people already invested, but these properties are antithetical to the process of development.
Most successful system seem to share a common history. A developer builds something simple that works, then has the freedom to tweak and optimize
for utility. That doesn't seem to happen when the building blocks require a complex chain of consent.

Option 3, the supernet, is the option that makes sense. It prevents data hosts from gaining all the leverage as they have in our current system. And they don't encode scarcity or consent as a core principle in the way blockchains do.

This era of digital dictatorships and fiefdoms needs to end.
Help speed things along.
Support the project by [registering](#LNS) a name.

# LNS{#LNS}

LNS is built on top of linkspace. It reads and writes entries to the 'lns' domain.  
It provides a way for naming groups and public keys.  
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
It is mostly working working, but I plan to build a dedicated exchange method for fast lookup of unknowns (recursive UDP similarly to DNS).
You can take a look on how it integrates with linkspace in the [guide](./docs/guide/index.html#ABELNS).
Registrations are open.
Read this page to find out how.

### What does a registration do?

It gives you binding right for all sub registration.
That means if you register under @:yourcompany:com you can set up:

1) a key addressed with the name @:yourcompany:com
1) a group name #:yourcompany:com
1) **all** authorities for names ending with *:yourcompany:com

I.e. because you own yourcompany:com, it can set up bindings for sales:yourcompany:com.
If you set up an authority for sales:yourcompany:com it can bind a public key to @:bob:sales:yourcompany:com.

### Do I require a registration?

Nothing in linkspace requires a registration.
Everything can be done without.
In fact, both :env and :local are meant for naming things for only yourself or between peers respectively and require no registration whatsoever.
Registrations allow you to pick a name to be known publicly for everybody.

### How can I register?

The following top level authorities have been assigned, and you can request a name ending in:

- :free. First come, first served.
- :dev if you have contributed to the code.
- :com for 10 euro per year.
- :nl for 10 euro per year.

To do so, scroll down to the end.

### Can I become a top level authority?

Yes. I am looking for people and organizations to do so.
Contact me at <antonsol919+registar@gmail.com> for more information.
If you represent a university you can get your name for free.

### Can I buy with crypto? Why not use crypto to do X, Y, Z ?

Linkspace is easy to integrate with blockchains (or even create a new blockchain).
You are free to build don'g it as you wish (MPL-2.0 license).
However, in an effort to put food on the table and pay taxes I prefer fiat money.
Additionally, 1 cent transactions save a lot of trouble w.r.t. identification if you lose the private key.

## Claim a name

This currently requires a some work.
To make a :free claim get the git repository, install rust and:

```terminal
make install-lk
lk --init key --key 'YOUR_NAME:local' | tee enckey
lk lns create-claim 'YOUR_NAME:free' --copy-from YOUR_NAME:local --until [now:+99Y] | tee lnsreq.lkp | lk p
```

Keep the file 'enckey' (and the password you entered) safe.
Email the lnsreq.lkp file to <antonsol919+lns@gmail.com>.
I plan to accept :free name requests until some clown automatically applies for all common names.
After which I'll probably put up a proof of work fence with some additional constraints on the name.

Get a pull request accepted, and you get a :dev name.

First come, first served[^3].

[^3]: You'll have to reply within a week once I send a follow-up email. This is done on a best effort basis - no rights are given.


# Download{#download}

Unzip, follow the ./linkspace-pkg/README.md to connect to a server.

- [linkspace-x86_64.zip](./download/linkspace-x86_64-unknown-linux-gnu.zip)
- [linkspace-aarch64.zip](./download/linkspace-aarch64-unknown-linux-gnu.zip)

The package contains:

- the `lk` CLI
- `lkpy.so` you can import with python.
- linkmail and imageboard domain applications.
- anyhost exchange .

## Git{#git}

Currently the primary repository is [github](https://github.com/AntonSol919/linkspace)

# Domains list{#domains}

- [linkmail](./docs/guide/index.html#linkmail) (available in the [download](#download))
- [imageboard](./docs/guide/index.html#imageboard) (available in the [download](#download))
- lns

# Groups{#groups}
## Known (public) servers{#server}

- [\#:test] - 83.172.162.31:5020

  This is the equivalent of unfiltered potato behind a proxy.

  It'll get purged every now and then but come say hi!

## Exchange Process{#exchange}

- [anyhost](./docs/guide/index.html#anyhost)

# About {#about}

Linkspace is currently an unfunded project, and is missing some key [features](https://github.com/AntonSol919/linkspace/blob/main/dev/TODO.md).
Meaning I still do other stuff for food, and won't be available all the time.

Please contact me directly if you're interested in supporting the project.
Or if you want to talk about specific applications.

### Contact

Of course the preferred way is to try and contact me on the test server.
For the less adventurous you can use <antonsol919@gmail.com>.

### Trademark

linkspace is a pending trademark of R.A.I.T. Solutions.
Once registered I will likely use similar terms as git uses for its trademark.
