# Explain Like I'm 5

This document is a simplified view of our web and how the linkspace protocol will change it.
The title is "Explain like I'm 5" but this is a lie.
5-year-olds will not know enough about the world to understand.
Somewhere between 15 and 40 should do. Still a challenge; 
The 15-year-old would be confused by analogies about printers or papers.
The 40y old has mastered the skill of "Print to PDF" and resists learning anything new.

The concepts described here should improve how you understand the digital world.
Its both an over simplification, and still difficult.
But I believe it is the minimum you _need_ in order to understand what the digital world _is_ and where we can take it.
The consequences are for yourself to figure out.

## How does our current web work?

There are hosts; systems that provide information.  
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

Next, in the dark holes off the silicon where we trapped lighting to dance to our whims an idea will power on.  
The exact idea is irrelevant.  
We're interested in the general outline.  
What they have in common is they ```read_index <-> process <-> insert_index```;  
and interact with other ideas doing the same.

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
Your device might automatically pick a program to open the data with.  
But you can pick different programs, the data itself stays the same.  

```read_index``` can also read /hello/ and return:

```
[file1, presentation, otherfile, subdir/file2]
```

The things text "/hello/file1" and "/world/file2" go by many names.
Paths, URLs, Identifiers, channels, etc.
The important thing is they allow us to organize in a hierarchy of sorted names.
It's a useful middle ground between how computers work and how humans think.
I will call it a path.

The events the host receives and writes down are also written to the index.  
They are simply in order and flat.  
It is flat and ordered by time:

| Path           | Data                        |
|----------------|-----------------------------|
| log/event0000  | Alice shares a new image    |
| log/event0001 | Bob wants to buy a car      |
| log/event0002  | Charlie is advertising spam |
| log/event0003  | Dave opened our app        |
| log/event0004  | Alice is looking for a taxi |

## In The Loop

Now that you know what an index is we can talk about the ```read_index <-> process <-> write_index``` loop.

A process:

- will wait for something to happen to the index,
- do [read_index] any number of times,
- combine and [process] the data,
- do [write_index] any number of times.
- return to wait

A typical process is a utility for others.
It creates commonly used sorted lists in the index.
This makes it faster and easy to find specific stuff.
They might create:

| Path                        | Data                        |
|-----------------------------|-----------------------------|
| /by-person/alice/event0000  | Alice shares a new image    |
| /by-person/alice/event0004  | Alice is looking for a taxi |
| /log-without-spam/event0000 | Alice shares a new image    |
| /log-without-spam/event0001 | Bob wants to buy a car      |
| /log-without-spam/event0003 | Dave opened our app         |
| /log-without-spam/event0003 | Alice is looking for a taxi |

Note that its common to think of "moving" or "deleting" paths from the index.  
If you take one thing away it should be this:  
The index is explicitly **NOT** about "moving" or "deleting" entries.
We can pretend to do so in various ways.
But they do not translate well to reality.
By assuming it is possible to "move" and "delete" we miscommunicate about what is happening.
We invented them in a different time for a different world.

The closest real ability is to "forget".
Be warned that in a network it is hard to forget.
The effect of "moving" or "deleting" are achieved by creating new entries.
For instance, "log-without-spam" is a copy of "log" without "event0002".

This ```read_index <-> process <-> write_index``` loop describes the hosts you know.

- Timelines
- Popularity ranking / recommendation lists
- Supply and demand for price calculations
- Find the result for your search.

## The graphical user interface

Talking to the host is done in arcane incantations, channeled through lightning, hidden from view.
For guests like us, the browser and apps paint the shapes we like.
With them, we interact with the index of the host.

It feels like a 'place'.  
People naturally care about the place.  
Less so the hidden lightning.  
We've come to see them as one whole.  

But they are two different things.  
In theory we as guests can split the two.  
Take one experience and use it elsewhere.  
We've sadly stopped doing that.  
In part because the hosts want to have their place in and between our lives.

## What is a Hash?

A hash function reads something and creates a very big number.  
The number is special.  
If we both get the same number, we have read the same data.  

This is useful when we want to talk about data.
It is a unique name for things.
This way computers can compare things.

It is the difference between sending:

- Gigabytes of a video to compare locally
- ```Have you seen BmtvS303a3hcPF2OvtCcNAna0mW1mwUzgyGgSB84tZU ?```

Large hosts do this all the time.
They use multiple computers all over the world to make things faster.
It is too expensive for those computers to agree on the order that things happen.
But they can agree on the hash of what happened.

## What is a public key?

Most hosts provide guests with accounts.  
It identifies you to the host.  
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
- When alice and bob want to talk, this _specific_ host has to _always_ be there and validate their identity.

Public key cryptography provides a math solution.
Anybody at anytime can run an algorithm to pick two numbers.
A (PrivateKey, PublicKey) pair.
If you do it right, you will not pick the same number twice before the sun explodes (give or take a few big bangs)

The private key can create 'signatures' that proof that they were present when the public key was created.
Without the private key you can _not_ proof that you have created a public key.

With public key cryptography:
- The host no longer receives your secret password.
- If alice and bob want to talk, they need to share public keys once through _any_ host at _any_ time in the past.

# What is the Linkspace Protocol?

Now that we know about the index, hashes, and public keys,
we can talk about the future of the internet.
In linkspace all events:

- are hashed,
- have a group
- have a domain
- can have a path,
- can be signed with a public key.

The hash is generated: It allows everybody to talk about things without ambiguity.
The group is added from context, it signals the people you intend to share with (for example the #:pub public group)
The domain is chosen by a developer. It is put before the path. It signals what interface to use and what the paths mean.
The path is set by a domain application, so others can interact with events.
The signature is created from the user's public key.
It allows us to identify each other.

## Consequences

```read_index``` and ```write_index``` are democratized.

Hosting the index is no longer special.  
The user interface could operate in any group.  
No lockin, no lockout.  

That is not to say everything is shared all the time.  
A [process] might be done by dedicated systems.  
Special groups can be made with specific sharing rules.  
The user interface can set constraints.  

However, the public hosts that are just a place to share what guests create are fucked.
Instead of dictating rules, they must compete.
Guests can move to a better host and still talk with the people they know.

We might even combine guest systems and be our own hosts.

## Closing thoughts

In short, the time of digital dictatorships and fiefdoms is ending.
Please help kill them.
Support the project by [registering](./lns.html) a public key name.

Join the quest for world domination.
Consider this your invitation.  
  

[^1]: I'ts not the password, but a hash function run multiple times so i'ts not at risk of being copied if the index is compromised.
