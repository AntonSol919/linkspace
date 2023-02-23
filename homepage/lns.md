# LNS

LNS provides a way for publicly naming groups and public keys.
Groups look like:

- #:lns:org
- #:myfancystore:com

Pubkeys like:

- @:john:lns:org
- @:alice:my:fancystore:com

:com, :dev, :local and :free registars are reserved.  
To apply for a new top level registar contact me directly.  

LNS is not fully implemented yet.  
However, you can reserve a name.  
Read this page to find out how.  

## Q&A:

### What does my registratiton do?

It gives you binding right for all sub registration.  
The keys associated with ':org' can give 'company:org' authority.  
'sales:company:org' authority to name 'bob:sales:company:org', etc.  
Every registration can carry additional links & data.  
In the future further auxiliary information is likely to be included  

### Do I require a registration?
Nothing in linkspace requires a registration.  
Everything can be done without.  
It allows you to pick names to tell others.  

### How much is a registration?
:free is free. First come first served.  
:dev registrations are given to contributors.  
:com is 10 euro per year.  
For others read on  

### Can I become a top level registar?
Yes. I am looking for more partners.  
They are leased on a first come first served bases of a 1000 euro p/m (12000 euro a year).
Alternative deals are negotiable. Just ask.  
The income help funds development and I'll throw in some goodwill consulting.  

### Can I buy with crypto? Why not use crypto to do X, Y, Z ?

Linkspace is probably easy to integrate with blockchains (or even create a new blockchain).
You are free to built on it as you wish (MPL-2.0 license).
However, in an effort to put food on the table and pay taxes I believe
plain fiat government backed money is currently better suited for the job then
a token representing a stake in spent electricity.

## Reserve a name

To make a reservation clone this git repository, install rust and:

```terminal
make install-lk
lk --init key | tee lnskey
lk keypoint lns:{#:pub}:/request/com/YOUR_NAME > lnsreq.lkp
```

Keep the file lnskey safe.
Email the lnsreq.lkp file to AntonSol919+lns at gmail.com  
I will accept /request/free untill some asshole decides to claim the dictionary.
After which I'll probably make it a little more difficult.

Get a pull request accepted, and you can get a :dev name.
    
First come first served[^1].

[^1]: You'll have to reply within a week once I send a follow-up email. This is done on a best effort basis no rights are given.
