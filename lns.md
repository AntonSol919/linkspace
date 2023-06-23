# LNS{#LNS}

LNS is a convention to for naming things in linkspace.
It writes points in the lns domain to be exact.  

Things like groups and public keys are 32 bytes. They look like [b:HrwlM8KNA25F2nkjLzU6exrKdXcI3TCH5ZseeSyIMrI].
This is unreadable for humans.

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

The top level names :local, ,:~ (or :file) , :env, and :pub are special.
You can take a look on how it integrates with linkspace in the [guide](./docs/guide/index.html#ABELNS).

LNS is currently only partially operational.

Registrations for public names are open though.
Read this page to find out how to claim yours.

### What does a registration do?

It gives you binding right for all sub registration.
That means if you register under @:yourcompany:com you can set up:

1) a key addressed with the name @:yourcompany:com
1) a group addressed with the name #:yourcompany:com
1) a key to manage registrations for names ending with *:yourcompany:com

I.e. The key with the authority for yourcompany:com can create bindings for sales:yourcompany:com.
That registration can set up a binding, such as for the key @:bob:sales:yourcompany:com.

### Do I require a registration?

No.
Nothing in linkspace requires a registration.
Everything can be done without.
In fact, \*:~ names you set up for your own use (similarly to /etc/hosts), and \*:local are meant for names you share between peers.
Registrations allow you to pick an unambiguous name to be known publicly, and they support the project financially.

### Does it require money?

No, not necessarily.

Names ending with:

- :free - First come, first served. Free of charge.
- :dev - if you get a pull request accepted.
- :com - 10 euro per year.
- :nl - 10 euro per year.

### How do i register?

To do so, see [claim a name](#claim).

### Can I register a top level authority like ':org' or ':sex' etc?

Yes. I am looking for people and organizations to do so.
Contact me at <antonsol919+registar@gmail.com> for more information.

### Can I buy with crypto? Why not use crypto to do X, Y, Z ?

Linkspace is easy to integrate with blockchains (or can be used to create new blockchains).
You are free to build on it as you wish (MPL-2.0 license).
In an effort to put food on the table and pay taxes I prefer fiat money and a 1 cent transaction save a lot of trouble w.r.t. identification if you lose the private key.
Other top level authorities set their own price and how to pay it.

## Claim a name{#claim}

This currently requires some work.
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
