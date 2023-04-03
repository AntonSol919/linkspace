# LNS - Lovely Name System

LNS provides a way for naming groups and public keys.
Groups look like:

- #:pub
- #:myfancystore:com
- #:friendsofbob:local

Pubkeys like:

- @:john:lns:org
- @:alice:my:fancystore:com
- @:john:nl
- @:me:local

:com, :dev, :free, :local, :env, and :pub registars are special/reserved.  
The system is operational except for quick UDP resolution. 
You can take a look on how it integrates with linkspace in the [guide](./docs/guide/index.html#ABELNS).  
Public registrations are open.  
Read this page to find out how.  

## Q&A:

### What does a registration do?

It gives you binding right for all sub registration.  
That means if you register under @:yourcompany:com you can setup:

1) a key addressed with the name '@:yourcompany:com'
1) a group name '#:yourcompany:com**
1) **all** authorities and keys for names ending with :yourcompany:com

I.e. because you own yourcompany:com, it has binding rights to a key for sales:yourcompany:com
Which can create a binding between a public key and the name @:bob:sales:yourcompany:com
In addition to group id's and public keys, a registration can carry additional data, such as a host IP.

### Do I require a registration?

Nothing in linkspace requires a registration.  
Everything can be done without.  
In fact, both :env and :local are meant for naming things for only yourself or between peers respectively and require no registration whatsoever.  
Registrations allow you to pick a name to be known publicly for everybody.  

### How much is a registration?

:free is free. First come, first served.  
:dev registrations are given to contributors.  
:com is 10 euro per year.  
For others read on  

### Can I register a name:org, name:nl, name:xxx or other another name not yet mentioned?
For country codes, and other popular top level names not yet mentioned as special/reserved I'll accept registrations and sign them.
However, the goal is to give/sell these top level registars to other organizations.
In other words, your registration might be dropped or re-assigned depending on how it is managed.

### Can I become a top level registar?

Yes. I am looking for people and organizations to do so.
Contact me at antonsol919+registar at gmail.com for more information.
If you represent a university you can get your name for free.

### Can I buy with crypto? Why not use crypto to do X, Y, Z ?

Linkspace is easy to integrate with blockchains (or even create a new blockchain).
You are free to built on it as you wish (MPL-2.0 license).
However, in an effort to put food on the table and pay taxes I prefer fiat money.
Additionally, 1 cent transactions save a lot of trouble w.r.t. identification if you lose the private key.
If all you want is to fund the project contact me directly (see below).

## Claim a name

To make a :free claim get the git repository, install rust and:

```terminal
make install-lk
lk --init key --key 'YOUR_NAME:local' | tee enckey
lk lns create-claim 'YOUR_NAME:free' --copy-from YOUR_NAME:local --until [now:+99Y] | tee lnsreq.lkp | lk p
```

Keep the file (and the password you entered) 'enckey' safe.
Email the lnsreq.lkp file to AntonSol919+lns at gmail.com
I plan to accept :free names until some clown decides to claim the dictionary.
After which I'll probably make it a little more difficult.

Get a pull request accepted, and you get a :dev name.

First come, first served[^1].

[^1]: You'll have to reply within a week once I send a follow-up email. This is done on a best effort basis - no rights are given.
