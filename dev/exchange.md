# The problem

A device runs a number of exchange processes.
Together they have to solve the following problems:

Any set of group members in any network topology have to efficiently
exchange the minimum set of packets that satisfy the domain applications request
it makes for the user.

For example, the following usecases should be supported.

- A group of a few dozen friends sync everything with eachother.
- A public message board. Multiple hosts carry part of the data.
- A company creates a network of dedicated servers for global distribution sharding on the pkt hash.
- A smart meter on the local LAN starts dumping its state as keypoint's on a UDP broadcast channel.

Idealy 'membership' and 'pull constraints' are integrated into the solution.

The 'full sync' is trivially correct.
The exchange does not care what the domain app's want to pull.

The public message board is probably decently solved once 'anyhost' is fully implemented with the bloom filter.

The company example i'll leave for others to design.
Its possible to skip bloom filters and do other distributed algorithms far faster.

The smart meter is trivial.

To implement 'membership' and 'pull/push constraints' is still an open question.
We'll have to figure out a set of conventions that makes it managable.
See exchange-limit.md for some thoughts on pull/push constraints.

Furthermore, there might be some rules we should set around the ubits and stamp net header fields.

We'll have to see how domain applications develop.
