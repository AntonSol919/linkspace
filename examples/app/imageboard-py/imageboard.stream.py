#!/bin/env python3
from lkpy import *
import os
import sys
if len(sys.argv) < 2:
    sys.exit('Usage: boardname ?stamp')

boardname = sys.argv[1]
create_stamp = int(sys.argv[2]) if len(sys.argv) > 2 else 0


lk = lk_open(create=True)
status = []
lk_status_poll(lk,
               lambda pkt: status.append(pkt) or False, # we're only interested in any reply. (We could check the data for OK)
               timeout=lk_eval("{s:+10s}"),
               domain=b"exchange",
               group=PUBLIC,
               objtype=b"process")

lk_process_while(lk)

if len(status) == 0:
    sys.exit("No exchange process active?")
else:
    print("Exchange status: ");
    print(str(status[0].data,'utf8'))


query = lk_query()
query_string = """
group:=:{#:pub}
domain:=:imageboard
path:=:/{0}
create:>=:{now:-1D}
:watch:{0}
"""
lk_query_parse(query,query_string,argv=[boardname])

#the exchange process is responsible to gather the data
lk_pull(lk,query)

#we just wait for every packet and redraw the painting starting at the 'create' stamp
script_dir = os.path.dirname(os.path.realpath(__file__))
os.system(f"{script_dir}/imageboard.view.py {boardname} 0")

def update_img(pkt):
    create = lk_eval2str("{create:str}",pkt)
    os.system(f"{script_dir}/imageboard.view.py {boardname} {create}")

lk_query_parse(query,"i_index:<:{u32:0}") # we only care for new stuff
lk_watch(lk,query, update_img)
lk_process_while(lk)
