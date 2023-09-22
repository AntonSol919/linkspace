# Linkspace

Linkspace is a open-source MPL-2.0 library and protocol to build event-driven applications using a distributed log with bindings for Python, JS/Wasm, and Rust.

---

Using a log of events as the source of truth is good design.
As evident by the success of tools like Kafka and Git, and its proliferation in the backend of databases & filesystems.

Properly used: Events creation can be distributed, algorithms/structures are made to consider what network conditions they can handle, and state becomes reproducable.

Many log systems operate behind closed walls by necessity.
Each event producer must be trusted and event authenticity is implied.
In linkspace these properties are explicit.

Linkspace uses Blake3 to hash events, optionally signed with Taproot Schnorr, and can link to older events.
This creates per event authenticity, that can expand, relate, and combine with other authenticity semantics required by an application.

Finally, events are partitioned by group (the set of recipients) and domain (the type of application).

The goal is to expand the log paradigm into the front-end and across organizational boundaries.

## Project status

The packet format is stable and stays readable in all future versions.
The API is mostly stable but can have small breaking changes between versions.

Any questions, feedback, or contributions are welcome!

## Links

See [Quick Start](https://www.linkspace.dev/code_intro.html) for a bash introduction to the packet format and using the cli.
The [Guide](https://www.linkspace.dev/guide/index.html) goes into more depth on the API and technical design.
[Tutorials](https://www.linkspace.dev/tutorial/index.html) has some annotated examples.


## Using linkspace

[Download](https://github.com/AntonSol919/linkspace/releases) the latest binary package to try out some examples.

### As a library 

All bindings follow the same basic API explained in the [Guide](https://www.linkspace.dev/guide/index.html)

- `cargo add linkspace --git "https://github.com/AntonSol919/linkspace"` - disable default features to compile for --target wasm32-unknown-unknown
- `pip install linkspace`
- `npm add linkspace-js` - Minimal JS bindings to read/write packets (including enckeys)



