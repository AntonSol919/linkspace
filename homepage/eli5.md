### A 15-second pitch

You should leave those internet platform you think you depend on.
Those middlemen providing you with timelines, feeds, markets, forums, etc.
They're troublesome and dangerous precisely because you can't leave.
The trade in the Terms Of Service is absurd.
You should leave.

...

Now take all those reasons "why I can't" that just entered your mind and forget them.
It is an illusion.
Eventually The Linkspace Protocol will make it feasible that you'll press a button and everything will keep working like you expect.
Except that you, and the people and organizations you trust, will be administrators. Operating on your terms.

# Explain Like I'm 5

This document is a simplified view of our web and where the linkspace protocol fits in.
Its title is "Explain like I'm 5" but this is a lie.
5-year-olds will not know enough about the world to understand.
Somewhere between 15 and 40 should do. Still a challenge. 
The 15-year-old would be confused by analogies about books and libraries, and has no reference for what came before the internet.
The 40-year-old has mastered the skill of "Print to PDF" and naturally resists change.

The goal is to give a basic model for thinking about the digital space in case you percieve it as a collection of 'apps'.
If you're already comfortable working with git or building webservers, you can jump into the [Guide](./docs/guide/index.html).

## How does our current web work?

There are hosts; systems that collect and provide information.  
There are guests; you in an app or the web-browser.  

A host has set up a process on a computer.
It will receive new information now and then.
This data is saved in order of receiving it. 

For example:

- Alice shares a new image
- Bob wants to buy a car
- Charlie is advertising spam
- Dave opened our app
- Alice is looking for a taxi

The host is running processes that operate on the events. 
The exact goal, programming language, or how it is orchestrated is irrelevant. 
Their basic functionality revolves around ```read_index <-> process <-> insert_index```;  

## The index

If you ever worked with files and folders you know the basics.  
An index looks something like this:

| Path               | Data        |
|--------------------|-------------|
| /work/file1        | Hello world |
| /work/presentation | ...          |
| /work/otherfile    | ...          |
| /work/subdir/file2 | ...          |
| /private/file1     | ...          |
| /private/file2     | ...          |

The ```read_index``` reads /work/file1 and returns ```Hello world```.  
Your device automatically picks a program to open the data with.  
But you can pick a different program to use the data in. 

These text things "/hello/file1" and "/world/file2" go by many names.
Paths, URLs, Identifiers, channels, etc.
The important thing is they allow us to organize in a hierarchy of sorted names.
We didn't always use this idea. 
But it has been extremely succesfull. 
It's a useful middle ground between how computers work and how humans think.
I will call it a path.

The events the host receives are also written to the index.  
They are simply in-order of time, and without nested entries.

| Path          | Data                        |
|---------------|-----------------------------|
| log/event0000 | Alice shares a new image    |
| log/event0001 | Bob wants to buy a car      |
| log/event0002 | Charlie is advertising spam |
| log/event0003 | Dave opened our app         |
| log/event0004 | Alice is looking for a taxi |

## In The Loop

Now that we have the index we can talk about the ```read_index <-> process <-> write_index``` loop.

A process:

- will wait for something to happen to the index,
- [read_index] any number of times,
- [process] and combine data,
- [write_index] any number of times.
- return to wait

A typical process is a utility for others.
What it writes, other processes will read. 

This makes it faster and easy to find specific stuff.
For instance, they can create: 

| Alice tracker              | Data                        |
|----------------------------|-----------------------------|
| /by-person/alice/event0000 | Alice shares a new image    |
| /by-person/alice/event0004 | Alice is looking for a taxi |

| Spam filter process         | Data                        |
|-----------------------------|-----------------------------|
| /log-without-spam/event0000 | Alice shares a new image    |
| /log-without-spam/event0001 | Bob wants to buy a car      |
| /log-without-spam/event0003 | Dave opened our app         |
| /log-without-spam/event0003 | Alice is looking for a taxi |

It is common to think of deleting or moving data.  
This is flawed.  

The effect of "moving" or "deleting" is done by creating new entries.
For example; the log-without-spam is a copy of log with event0002 deleted.

The features you're familiar with in many of your apps are implemented through this loop.
Things like:

- The timeline of posts
- Popularity ranking / recommendation lists
- Supply and demand for price calculations
- Find the result for your search.

## The graphical user interface

The app you use to talk to the host has two parts. 
The painting of buttons or texts, and the communication in ones and zeros.

The designer works hard to make it feel like a 'place'.  
A single whole you should care about.  

But they are two different things.  
The hosts trade you this comfort for a perpetual grant to exploit you and your communication.  
Or it is as they claim; for your own protection because you are a child.  

## What is a Hash?

A hash function reads something and creates a very big number.  
The number is special.  
If we both get the same number, we have read the same data.  

This is useful when we want to talk about data.
It is a practically unique for every piece of data. 
With this number multiple computers can compare things.

It is the difference between sending:

- Gigabytes of a video to compare locally
- ```Have you seen BmtvS303a3hcPF2OvtCcNAna0mW1mwUzgyGgSB84tZU ?```

Large hosts do this all the time.
They use computers spread out over the world to make things faster.
It is too expensive for those computers to agree on the order that things happen.
But they can agree on the hash of what happened.

## What is a public key?

Hosts usually give the option to get an account.
It identifies you to the host, and somethimes to others. 
It's very simple to implement if nobody can access the index.  

| Path                  | Data             |
|-----------------------|------------------|
| /account/alice/pass   | \<password\>[^1] |
| /account/alice/email  | ..               |
| /account/alice/phone  | ..               |
| /account/bob/pass     | ...              |
| /account/charlie/pass | ...              |

Alice can proof herself to the host, and so can bob.
But this is archaic in two ways.

- We are sending the host the password when we register
- If alice and bob want to know for sure they are talking to eachother, this _specific_ host has to _always_ play middleman to validate their identity.

Public key cryptography provides a math solution.
Anybody at anytime can run an algorithm to pick two numbers.
A (PrivateKey, PublicKey) pair.
If you do it right, you will not pick the same number twice before the sun explodes (give or take a few big bangs)

The private key can create 'signatures' that proof that they were present when the public key was created.
Without the private key you can _not_ proof that you have created a public key.

With public key cryptography:

- The host no longer receives a secret password.
- If alice and bob want to know for sure they are talking to eachother, they need to share public keys once through _any_ host at _any_.

# What is the Linkspace Protocol?

Now that we know about the index, hashes, and public keys, we can talk about what the internet can be.

In linkspace all events:

- are hashed,
- have a group
- have a domain
- can have a path,
- can be signed with a public key.

The hash is generated: It allows everybody to efficiently talk _about_ things.
You set a group, it exists because we say it does, and your device exchanges with the members of that group.
The domain is chosen by a developer. It signals what app to use, and how the events are organized.
The signature is created from the user's public key.
It allows us to identify each other.

Any sequence of events can be merged.

## Consequences

```read_index``` and ```write_index``` are liberated.

It can be moved; hosted where it makes sense to the user.

That is not to say everything is shared all the time.
A [process] can be done on specialized systems.
Groups can be made with specific sharing rules.

However, we change the concept of a public 'platforms'.  
Their current value as _just_ a place to connect people is gone.  
Instead of dictating rules, they must compete.  
Provide value in a fair trade to the user.  
If not, another can take over without breaking what already is.  

We also get to build new (social) contracts:  
Discussions on scientific papers _about_ the data and code that is linked in.  
Social Media apps working over Bluetooth, Wi-Fi, Radio etc.  
Public announcements signed with keys are required to spot the AI generated lies.  
In-house communication tools, expandable by any developer.  
A taxi or hotel 'markets' insured and operated by locals at a fraction of the current markup,  
Gradual expansion of the interop in supply chains.  

With far less complexity than you're expecting[^2].

## Closing thoughts

The Linkspace Protocol is free for everybody, anytime, anywhere, forever.

The time of digital dictatorships and fiefdoms is ending.
Please help kill them.
Support the project by [registering](./lns.html) a public key name.

[^1]: Techinically this is not the password, but a hash function run multiple times so if done right it is not at risk if the index is copied.
[^2]: [guide#state-complexity](./docs/guide/index.html#state-complexity) - but it will take time before it is simple for users.

