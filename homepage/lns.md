# LNS

LNS provides a way for publicly naming groups and public keys.
Groups look like:

- #:pub
- #:myfancystore:com
- #:friendsofbob:local

Pubkeys like:

- @:john:lns:org
- @:alice:my:fancystore:com
- @:john:local
- @:me:local

:com, :dev, :free, :local, and :pub registars are special/reserved.  
To apply for a new top level registar contact me directly.  

LNS is not fully implemented yet.  
You can take a peek on how it will integrate with linkspace in the [guide](./docs/guide/index.html#ABELNS).  
Reservations are open.  
Read this page to find out how.  

## Q&A:

### What does a registratiton do?

It gives you binding right for all sub registration.  
The keys associated with ':org' can give 'company:org' authority.  
'sales:company:org' authority to name 'bob:sales:company:org', etc.  
Every registration can carry additional links & data.  

### Do I require a registration?

Nothing in linkspace requires a registration.  
Everything can be done without.  
You can setup a root [#:...:local] key simple enough.  
Registrations allow you to pick a name to be known publicly for everybody.  

### How much is a registration?

:free is free. First come first served.  
:dev registrations are given to contributors.  
:com is 10 euro per year.  
For others read on  

### Can I become a top level registar?

Yes. I am looking for people to do so.
Top level names are leased on a first come first served bases, no constraints, for a 1000 euro p/m (12000 euro a year).
Alternative deals are negotiable. Just ask.  
I'll throw in some goodwill consulting if you ever decide to disrupt some industry.  

### Can I buy with crypto? Why not use crypto to do X, Y, Z ?

Linkspace is easy to integrate with blockchains (or even create a new blockchain).
You are free to built on it as you wish (MPL-2.0 license).
However, in an effort to put food on the table and pay taxes I believe
plain fiat government backed money is currently better suited for the job then
a token representing a stake in spent electricity.
Additionally, 1 cent transactions saves a lot of trouble w.r.t. account recovery.
If all you want is to fund the project contact me directly (see below).

## Reserve a name

To make a reservation clone this git repository, install rust and:

```terminal
make install-lk
lk --init key | tee lnskey
lk keypoint lns:[#:pub]:/request/com/YOUR_NAME > lnsreq.lkp
```

Keep the file lnskey safe.
Email the lnsreq.lkp file to AntonSol919+lns at gmail.com  
I will accept /request/free until some asshole decides to claim the dictionary.
After which I'll probably make it a little more difficult.

Get a pull request accepted, and you get a :dev name.

First come first served[^1].

[^1]: You'll have to reply within a week once I send a follow-up email. This is done on a best effort basis no rights are given.
