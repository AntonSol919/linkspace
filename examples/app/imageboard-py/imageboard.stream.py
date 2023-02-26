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
"""
lk_query_parse(query,query_string,inp=[boardname])

print(lk_query_print(query))
