# LNS - Lovely Name System

LNS provides a way for naming groups and public keys.  
**Groups** look like:

- #:pub
- #:myfancystore:com
- #:friendsofbob:local

**Public keys** look like:

- @:john:lns:org
- @:alice:my:fancystore:com
- @:john:nl
- @:me:local

The top level names :local, :env, and :pub are special.
The system is mostly working except for quickly resolving them over UDP.
You can take a look on how it integrates with linkspace in the [guide](./docs/guide/index.html#ABELNS).
Registrations are open.
Read this page to find out how.

### What does a registration do?

It gives you binding right for all sub registration.
That means if you register under @:yourcompany:com you can set up:

1) a key addressed with the name @:yourcompany:com
1) a group name #:yourcompany:com
1) **all** authorities for names ending with *:yourcompany:com

I.e. because you own yourcompany:com, it has binding rights to a key for sales:yourcompany:com.
Which can create a binding between a public key and the name @:bob:sales:yourcompany:com.

### Do I require a registration?

Nothing in linkspace requires a registration.
Everything can be done without.
In fact, both :env and :local are meant for naming things for only yourself or between peers respectively and require no registration whatsoever.
Registrations allow you to pick a name to be known publicly for everybody.

### How can I register?

The following authorities have been assigned, and you can request a name ending in:

- :free. First come, first served.
- :dev if you have contributed to the code.
- :com for 10 euro per year.

To do so, scroll down to the end.
For others read on.

### Can I register a name:org, name:nl, name:xxx or other another name not yet mentioned?

For country codes, and other popular top level names not yet mentioned I'll accept registrations and sign them for 10 euro per year.
However, I intend to give/sell these top level binding rights to other organizations.
In other words, I can't guarantee they'll remain valid.

### Can I become a top level authority?

Yes. I am looking for people and organizations to do so.
Contact me at <antonsol919+registar@gmail.com> for more information.
If you represent a university you can get your name for free.

### Can I buy with crypto? Why not use crypto to do X, Y, Z ?

Linkspace is easy to integrate with blockchains (or even create a new blockchain).
You are free to built on it as you wish (MPL-2.0 license).
However, in an effort to put food on the table and pay taxes I prefer fiat money.
Additionally, 1 cent transactions save a lot of trouble w.r.t. identification if you lose the private key.
If all you want is to fund the project contact me directly.

## Claim a name

This currently requires a some work.
To make a :free claim get the git repository, install rust and:

```terminal
make install-lk
lk --init key --key 'YOUR_NAME:local' | tee enckey
lk lns create-claim 'YOUR_NAME:free' --copy-from YOUR_NAME:local --until [now:+99Y] | tee lnsreq.lkp | lk p
```

Keep the file 'enckey' (and the password you entered) safe.
Email the lnsreq.lkp file to <AntonSol919+lns@gmail.com>.
I plan to accept :free name requests until some clown automatically applies for all common names.
After which I'll probably put up a proof of work fence with some additional constraints on the name.

Get a pull request accepted, and you get a :dev name.

First come, first served[^1].

[^1]: You'll have to reply within a week once I send a follow-up email. This is done on a best effort basis - no rights are given.
