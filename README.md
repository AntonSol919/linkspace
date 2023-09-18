# Linkspace 

Linkspace is a open-source MPL-2.0 library and protocol to build event-driven applications using a distributed log with bindings for Python, JS/Wasm, and Rust.

--- 

Using a log of events as the source of truth is good design. 
As evident by the success of tools like Kafka and Git, and its proliferation in the backend of databases & filesystems.

Properly used: Events creation can be distributed, algorithms/structures are made order-irrelevant and idempotent, and thus state becomes reproducable. 

Many log systems operate behind closed walls by necessity. 
Each event producer is trusted and event authenticity is implied. 
In linkspace these properties are explicit.

Linkspace uses Blake3 to hash events, optionally signed with Taproot Schnorr, 
and events support linking to multiple other events. 
This provides basic authenticity that can be expanded upon by an application.

Furthermore, linkspace has a novel group:domain separation, is fast, and is designed to have a small API.

The goal is to expand the log paradigm into the front-end and across organizational boundaries.

## Project status

The packet format is fixed and will stay readable in all future versions.
The API is mostly stable but will likely have small breaking changes. 
The documentation and examples are a work in progress.

Any questions, feedback, or contributions are welcome!

## Links

See [Quick Start](https://www.linkspace.dev/code_intro.html) for a bash introduction to the packet format and using the cli.
The [Guide](https://www.linkspace.dev/docs/guide/index.html) goes into more depth on the API and technical design.
[Tutorials](https://www.linkspace.dev/docs/tutorial/index.html) has some annotated examples.

[Download](https://github.com/AntonSol919/linkspace/releases) the latest release or clone from [GitHub](https://github.com/AntonSol919/linkspace)


## Using linkspace

You can use linkspace through 

- The `lk` CLI 
- The Rust library with `linkspace = {git = "https://github.com/AntonSol919/linkspace"}` (disable default features to compile to wasm)
- Python [bindings](https://pypi.org/project/linkspace/) with `pip install linkspace`. 
- Javascript bindings in `ffi/linkspace-js` (WIP: Is missing the runtime api)

For building from source see [Guide#setup](https://www.linkspace.dev/docs/guide/index.html#setup) or the README.md's.

[Clone & build](https://github.com/AntonSol919/linkspace) or [Downloading](https://github.com/AntonSol919/linkspace/releases) prebuild binaries to give it a try.
Check out the `./emulate` folder for running locally or try and connect to a public test instance with the `./join-testexchange` script. 

