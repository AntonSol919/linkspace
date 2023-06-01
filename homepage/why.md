# Why?{#why}

This protocol came to be for two reasons.

Reason 1: I found it overly complex to create, share, and try ideas for multi-user applications.
Building something new should be fun.
Build and manage servers, user accounts, groups, etc is not the fun part.

[Reason 2]{#reason2}: I wanted to have an answer to a key question that underpins how our society perceives and uses digital systems.

**Who gets to __administrate__ the cat videos, taxi service, messages, and other data we interact with online?**

As I see three options:

1. Applications are hosted by one organization on their machines. This is how most of the internet currently works.
2. A pay2play scheme, on a blockchain.
3. Users pick their admins, on a supernet.

In Option 1, our current web, the host is the administrator.
It has unilateral control because it relays data between users.
Furthermore, many popular hosts are the result of an environment where winner keeps all.
They're built for optimal exploitation of their users.
Everything is permitted to keep it that way.
**Lock-in** the users, and **lock-out** any threat to the platform's place in your life.

You must break these locks.

[Option 2]{#option2}, blockchains.
They are hyped to be many things, and some people believe blockchains will be the foundation of our digital space going forward.
I don't see how.
They encode scarcity. Using them is made to be expensive.
Very few systems require that.
Appointing and trusting an administrator has always been cheaper and simpler.

Scarcity makes blockchains attractive for the people already invested, but these properties are antithetical to the process of development.
Successful systems are build from small incremental improvements.
That dynamic doesn't seem to take place when the building blocks are a costly chain of consent.

Option 3, a supernet like linkspace, is the option that makes more sense to me.
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

### Isn't the database and packet fields to much overhead?
If all you want to do is stream one movie from a single host, and forget it then linkspace might be too much overhead.
Few projects stay that simple. Most projects grow in scope to identify users, save their comments, add them to groups, scale beyond a single server. 
Once a full stack is build, linkspace can be a thin alternative.
Furthermore, it is designed to be fast/low energy, such that you can stream a video on a potato phone.
`dd bs=10G count=1 if=/dev/zero | lk data > /dev/null`

### Can you ask people to deal with the added complexity?

Yes.
The nature of communication over distance is autonomous and asynchronous so the complexity is not artificial/accidental.
Linkspace might lack 3 decades of tooling that made the web relatively easy for users, but that alone isn't a reason to stay with it.
Furthermore, the number of configuration ends up smaller: Passwords, Groups, friends can be setup once and used by every application.
Finally, giving people responsibility isn't a bad thing.
Just like teaching people to read, write, and do maths isn't a bad thing.
Teaching them to control a digital space is a logical next step.

### Isn't it a good thing that central hosts administrate what I and others can see online?:
You can still outsource this to third parties.
Making it an integral part of how billions of us spend hours each day communicating should scare the shit out of everyone.

### Won't it devolve to the same paradigm of centralized systems?:
Maybe, maybe not. If a users can walk away from a host platform without losing their history, the host has to give a better deal than they do now.

## Why not <alternative>?

- It has either hash addresses or custom url addresses, not both.
- Too slow. A system should be able to stream video, not configure a different protocol to do so.
- No Groups. Consequently there is no or little granularity in what you share.
- No domains. Consequently the barrier to quickly hack something together is high. A developer might fear messing up.
- Its not a full supernet, but just a blockchain. I.e. A singular log of packets with links to its previous entry that hold accounting data.
- The wrong order of defining the system's components.
  - A blessed/fixed method of exchanging data, instead of a external/modular system.
  - An ever growing set of stream protocols to negotiate a state.

Take your pick. That does not alternative are worse.
More than one supernet can co-exist, and they have their own strong points.
