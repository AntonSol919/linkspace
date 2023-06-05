# Why?{#why}

This protocol came to be for two reasons.

Reason 1: I found it overly complex to create, share, and try ideas for multi-user applications.
Building something new should be fun.
Build and manage servers, user accounts, groups, etc is not the fun part.

[Reason 2]{#reason2}: I wanted to have an answer to a key question that underpins how our society perceives and uses digital systems.

**Who gets to __administrate__ the cat videos, taxi service, messages, and other data we interact with online?**

As I see two options:

In Option 1, our current web, the host is the administrator.
It has unilateral control because it relays data between users.
Furthermore, many popular hosts are the result of an environment where winner keeps all.
They're built for optimal exploitation of their users.
Everything is permitted to keep it that way.
**Lock-in** the users, and **lock-out** any threat to the platform's place in your life.

You must break these locks.

[Option 2]{#option2}, we use cryptographic hashes and public keys to model a layer that trancends the internet.

In a supernet like linkspace, is the option that makes more sense to me.
It splits up the hosts and doesn't encode scarcity as a core principle.

In linkspace the role of host as we know them today is split up into:

- Providing groups with servers to relay data.
- Developing domain applications that run on linkspace.

Should someone in the system abuses their position the user can decouple and change things up without losing access to their history and relations to others.
If for nothing else, a supernet is worth it to get us more useful competition.

Whether this all works out as intended, I have no idea.

My hope is we can look back at the current era of the internet and recognize it for what it is.
digital fiefdoms.
The step forward is to take full control over who lords over us.

Help speed things along.
Try it out, get involved, and build new stuff ([tutorials](./docs/tutorial/index.html)).
Support the project financially by registering a non-free LNS name.


# Q&A

Some common questions and answers about the project in general:

### Is a supernet, like linkspace, a blockchain ?

Blockchains and supernets share a common vision:

Using cryptographic hashes and public keys to provide people with a model of data that trancends the internet.

Blockchain are data put into 'blocks' which are chained together over time to created (centralized) consensus.
A blockchain's goal is to provide a model and tools to make consensus simple.

However, (centralized) consensus isn't that useful most of the time.
(Beyond the obvious financial incentives from selling exclusivity)

A supernet's goal is to provide a model and tools to make operating on distributed state simple.

### Isn't the database and packet fields to much overhead?
If all you want to do is stream one movie from a single host, and forget it then linkspace might be too much overhead.
Few projects stay that simple. Most projects grow in scope to identify users, save their comments, add them to groups, scale beyond a single server. 
Once a full stack is build, linkspace can be a thin alternative.
Furthermore, it is designed to be fast/low energy, such that you can stream a video on a potato phone.
`dd bs=10G count=1 if=/dev/zero | lk data > /dev/null`

### Can you ask people to deal with the added complexity?

Yes.

The nature of communication over distance is chaotic and asynchronous, so the "complexity" is not artificial or accidental.
Linkspace might lack 3 decades of tooling that made the web relatively easy for users, but that alone isn't a reason to stay with it.
Furthermore, the number of configuration ends up smaller: Passwords, Groups, friends can be setup once and used by every application.

Finally, making people responsible isn't a bad thing, and impotence can be worse.

### Isn't it a good thing that central hosts administrate what I and others can see online?:
You can still outsource this to third parties.
Making it an integral part of how billions of us spend hours each day communicating should scare the shit out of everyone.

### Won't it devolve to the same paradigm of centralized systems?:
Maybe, maybe not. If a users can walk away from a host platform without losing their history, the host has to give a better deal than they do now.

## Why not <alternative>?

- It has either hash addresses or custom url addresses, not both.
- Too slow. We should stream video as is, not hand it over to different protocol.
- No Groups. Consequently there is no or little granularity in what you share.
- No domains. Everything becomes one app.
- Its distracted with building universal concensus (blockchains), instead of focusing on the utility without a consensus.
- The wrong order of defining the system's components.
  - A blessed/fixed method of exchanging data, instead of a external/modular system.
  - An ever growing set of stream protocols to negotiate a state.
  - HTTP servers extended and cooreced to act fededrated, instead of HTML coerced to be a UI for a federated network protocol.

Take your pick. That does not alternative are worse. 
These are goals and properties that I value.
More than one supernet can co-exist, and have different strong points.
