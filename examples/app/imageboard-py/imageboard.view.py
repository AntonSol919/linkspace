#!/bin/env python3.11
from lkpy import *
import os
import sys
if len(sys.argv) < 2:
    sys.exit('Usage: boardname ?stamp')

boardfile = sys.argv[1] + ".png"
boardname = sys.argv[1]
create_stamp = int(sys.argv[2]) if len(sys.argv) > 2 else 0

if not os.path.exists(boardfile):
    os.system(f"magick convert -size 1000x1000 xc:transparent PNG32:{boardfile}")

lk = lk_open(create=True)

# You can parse multiple statements as abe.
# The usual ABE context is available, and you can extend it with argv
query_string = """
group:=:[#:pub]
domain:=:imageboard
path:=:/[0]
create:>=:[1/u64]
"""
query = lk_query_parse(lk_query(),query_string,argv=[boardname,str(create_stamp)])
# or use templates. if you're just interested in the string
query = lk_query_parse(query,f"create:>=:[:{str(create_stamp)}/u64]")

# Or if you have the exact bytes
create_b = create_stamp.to_bytes(8,byteorder='big')
query = lk_query_parse(query,f"create:>=:[0]",argv=[create_b])
# Or if you're only adding a single statement
query = lk_query_push(query,"create",">=",create_b)

# The query merges overlapping predicates, and errors on conflicting predicates

# Query parsing is somewhat forgiving in that it allows Group, Domain, and Path two syntax's
# Group can take the b64 no-pad string
# Domain is 16 bytes but does not have to prepend '\0'
# Path takes either a '/' delimited expression, or the 'spath' bytes ( as given by the spath function or pkt.spath value )
# The other values require the exact number of bytes, in big endian when a number.

# Its worth understanding why these two work. Checkout the guide
create_abe = f"[u64:{str(create_stamp)}]"
assert create_b  == lk_eval(create_abe)
assert lk_encode(create_b,"u64") == create_abe

# we'll collect our entries in here 
image_data = []
def update_image(pkt):
    create = pkt.create # all the links in the packet will have this as their z-index
    for link in pkt.links:
        x = int(str(link.tag[:8],'ascii'))
        y = int(str(link.tag[8:],'ascii'))
        q = lk_hash_query(link.ptr) # shorthand for :mode:hash-asc i:=:[u32:0] hash:=:HASH
        q = lk_query_push(q,"recv","<",lk_eval("[now:+3s]"))

        # we need a uniq id to register this query under.
        wid = bytearray(pkt.hash)
        wid.extend(link.ptr)
        q = lk_query_push(q,"","watch",bytes(wid))
        
        # print("Looking for ",lk_query_print(q,True))
        # we could get with 'lk_get(lk,q)' but to give new packets a chance to arrive we've set recv < now+3s so we will watch them.
        lk_watch(lk,q,lambda data_pkt : image_data.append([create,x,y,data_pkt,pkt]))

# we only care about the ones we know right now. 
lk_get_all(lk,query, update_image)

# Because we set a timeout ( recv<now+5s ) for all data packets ( in case we're still receiving them )
# we can simply wait until all callbacks are done or dropped.
lk_process_while(lk)

image_data.sort()

import pathlib
pathlib.Path("./fragments").mkdir(parents=True, exist_ok=True)

for [_,x,y,datap,parent] in image_data:
    filename = lk_eval2str("./fragments/[hash:str]",datap)
    try:
        with open(filename, "bx") as f:
            f.write(datap.data)
            f.flush()
            f.close()
    except Exception as e:
        pass
    print(f"placing at {x},{y} the image {filename}")
    os.system(f"magick composite -geometry +{x}+{y} {filename} PNG32:{boardfile} PNG32:{boardfile}")

