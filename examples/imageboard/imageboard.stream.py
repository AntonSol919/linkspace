#!/bin/env python3
from linkspace import *
import os
import sys
if len(sys.argv) < 2:
    sys.exit('Usage: boardname ?stamp')

boardname = sys.argv[1]
create_stamp = int(sys.argv[2]) if len(sys.argv) > 2 else 0


lk = lk_open(create=True)
ok = lk_status_watch(lk,
               qid=b"ex",
               timeout=lk_eval("[us:+2s]"),
               domain=b"exchange",
               objtype=b"process")
if not ok and lk_process_while(lk,qid=b"ex") == 0:
    sys.exit("No exchange process active?") # not strictly necessary, but otherwise pull does nothing
else:
    print("Exchange ok");


query_string = """
group:=:[#:pub]
domain:=:imageboard
spacename:=:/[0]
create:>=:[now:-1D]
:qid:[0]
"""
query = lk_query_parse(lk_query(),query_string,argv=[boardname])

#We signal the exchange process to gather the data
lk_pull(lk,query)

#we wait for every packet and redraw the painting starting at the 'create' stamp
script_dir = os.path.dirname(os.path.realpath(__file__))
os.system(f"{script_dir}/imageboard.view.py {boardname} 0")

def update_img(pkt):
    create = lk_eval2str("[create:str]",pkt)
    os.system(f"{script_dir}/imageboard.view.py {boardname} {create}")

query = lk_query_parse(query,"i_db:<:[u32:0]") # we only care for packets not currently in the database. The new stuff
lk_watch(lk,query, update_img)
lk_process_while(lk)
