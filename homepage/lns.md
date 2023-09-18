# LNS{#LNS}

LNS is an experimental convention to name things in linkspace.

It writes points in the lns domain to be exact.  

Things like groups and public keys are 32 bytes. They look like `b:HrwlM8KNA25F2nkjLzU6exrKdXcI3TCH5ZseeSyIMrI`.
This is unreadable for humans.

With LNS they can be assigned names.
Both publicly by registering, and privately for your own convenience.  

**Groups** look like:  

- \#:pub
- \#:myfancystore:com
- \#:friendsofbob:local

**Public keys** look like:  

- @:john:lns:org
- @:alice:my:fancystore:com
- @:bob:nl
- @:me:local

The top level names :local, :file , :env, and :pub are special.
You can take a look on how it integrates with linkspace in the [guide](./docs/guide/index.html#ABELNS).

### What does a registration do?

It gives you binding right for all sub registration.
If you register @:yourcompany:com you can set up:

1) a key addressed with the name @:yourcompany:com
1) a group addressed with the name #:yourcompany:com
1) a key to manage registrations for names ending with *:yourcompany:com

I.e. The key with the authority for yourcompany:com can create bindings for sales:yourcompany:com.
That registration in turn is able to set up new sub registrations, such as @:bob:sales:yourcompany:com.

### Do I require a registration?

No.
Its a public ( or private ) address-book.

### Does it require money?

The private will never cost money.
For a public registration it depends.
Each registration can set their own rules for sub registrations.
Some top level names can ask you to pay - others are free.

The current top level registries accepting names are:

- example:free - First come, first served.
- example:dev - Get a pull request accepted.
- example:com - commercial - 10$ per year and helps fund the project!

### How do I register?

This is currently still a manual process as its not a high priority.
However, you can pre-register your name to make sure its yours.

To make a :free claim get the `lk` and `linkspace-lns` utility from the download or build from source.

```terminal
source ./activate
lk --init key --key 'YOUR_NAME:local' | tee enckey
lk lns create-claim 'YOUR_NAME:free' --copy-from YOUR_NAME:local --until [now:+99Y] | tee lnsreq.lkp | lk p
```

Keep the file 'enckey' (and the password you entered) safe.
Email the lnsreq.lkp file to <antonsol919+lns@gmail.com>.

### Can I be a top level authority like ':nl' or ':org', etc?

Yes. I am looking for people interested to do so.
Shoot me a message at <antonsol919+registar@gmail.com>.
