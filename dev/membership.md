# Membership

An exchange process and domain both need to know which members the user considers part of a group.

This is somewhat related to exchange limits.

# Option 1

Local {root} signs an 'exchange:{#:0}:/GROUP/memberhips' packet with a its links set :

publickey : ...pubkey
publickey : ...pubkey
subgroup  : ...groupid
subgroup  : ...groupid

The members could follow along with a group 'admin' by something like
looking for packets signed in exchange:GROUP:/GROUP/membership by the 'admin' and rewriting them into exchange:{#:0}/GROUP/membership signed by {root}

# Option 2 
Full LNS integration. Everybody needs a keyname under @:some:prefix:nl
