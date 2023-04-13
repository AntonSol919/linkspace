from linkspace import *
from datetime import datetime

lk = lk_open("./private",create=True)
lk_process(lk)

q = lk_query()
lk_query_parse(q,"""
:id:test
:mode:log-asc
i_index:<:[u32:0]
""")

outer_l = []
inner_l = []

def inner(pkt):
    print("inner triggered",pkt)
    outer_l.append(pkt)

def outer(pkt):
    print("outer triggered",pkt)
    outer_l.append(pkt)
    lk_watch(lk,q,inner)

lk_watch(lk,q,outer)
lk_process_while(lk,until=lk_eval("[now:+1s]"))

lk_save(lk,lk_datapoint("one"+str(datetime.now())))
lk_save(lk,lk_datapoint("two"+str(datetime.now())))
lk_process_while(lk,until=lk_eval("[now:+1s]"))

print(outer_l,inner_l)
