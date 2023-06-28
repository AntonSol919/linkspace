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
- @:bob:nl
- @:me:local

The top level names :local, :file , :env, and :pub are special.
You can take a look on how it integrates with linkspace in the [guide](./docs/guide/index.html#ABELNS).

LNS is currently only partially operational.

But you can register a public name!

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
Its just a public or private phone-book.

### Does it require money?

That depends. The public LNS registries are meant to set their own rules. Some top level names can ask you to pay - others are free.

The current top level registries accepting names are:

- example:free - First come, first served.
- example:dev - Get a pull request accepted.
- example:com - commercial - 10$ per year and helps fund the project!

### How do I register?

This is currently still a manual process as its not a high priority and other parts is still missing.
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

Yes. I am looking for people willing to do so.
Shoot me a message at <antonsol919+registar@gmail.com> if you're interested.
