import os
import tempfile
import subprocess
import collections
import shlex
import functools
import cmd
from dataclasses import dataclass
from lkpy import *
from pathlib import Path


list_template = "[/~?:[pubkey]/@/b]:[path:str] = [data/slice::20/rpad:20: ] [create/s:delta/rfixed:18: ]:[hash/2mini] # [links_len:str]([:[/links:[tag:str] [ptr/2mini],]/~rcut:32])"
print_template= "==[hash]==\\n[/~?:[pubkey]/@/b]\\n[path:str]\\n[create/s:str]\\n[data:str]\\n==[links_len:str]==\\n[/links:\\t[tag:str]\\t[ptr/2mini]\\n]",


# tmporary dev init
path = "/home/rs/linkspace"
lk = lk_open(path,create=True)
keyname ="me:local"
password=""
group_name = "[#:pub]"
group=lk_eval(group_name)
baseq = lk_query()
lk_query_parse(baseq,f"domain:=:linkmail\ngroup:=:{group_name}")


keyname = keyname or input("Key name [local]> ") or "me:local"
try:
    print("Using key:",lk_eval2str(f"[@:{keyname}/?b]"))
except:
    print(keyname , " not found - we'll try creating it")
passw = password # getpass.getpass(prompt='Password> ', stream=None)
print("Opening key");
key = lk_key(lk,password=lk_eval(passw),name=keyname,create=True)
lk_process(lk) # required for lk_encode
keyname = lk_encode(key.pubkey,"@") # ensure we use the preferred name for this key
print(f"Key ok: ",keyname);

lk_keypoint = functools.partial(lk_keypoint,key=key,domain=b"linkmail",group=group)
lk_linkpoint = functools.partial(lk_linkpoint,domain=b"linkmail",group=group)



def tag_str(t):
    """strip nulls from 16 byte value and escape"""
    return lk_eval2str("[0/?a0]",argv=[t])

def links_str(links=[]):
    links = map(lambda l: tag_str(l.tag) + " " + lk_encode(l.ptr,"b"),links)
    return "\n".join(links)



def get_input(links = []):
    editor = os.environ.get('VISUAL', os.environ.get('EDITOR', 'vi'))
    with tempfile.NamedTemporaryFile(mode='w+b', delete=False) as tf:
        tf.write(links_str(links).encode())
        tf.write(b"\n==MSG==")
        tf.flush()
        # will not work on windows - must first close file 
        tmp_file = tf.name
        process = subprocess.Popen(shlex.split(f"{editor} \"{tmp_file}\"")) 
        process.wait()
    data = Path(tmp_file).read_text("utf-8").split("\n==MSG==\n",1)
    if len(data) == 1:
        return ([],data[0])
    def aslink(st):
        [tag,link] = st.split(maxsplit=1)
        return Link(lk_eval(f"[a:{tag}]"),lk_eval(link))
    links = list(map(aslink,data[0].splitlines()))
    return (links,data[1])

def get_exchange_status():
    status =[] 
    lk_status_poll(lk,
               lambda pkt: status.append(pkt),
               timeout=lk_eval("[s:+2s]"),
               domain=b"exchange",
               group=group,
               objtype=b"process",watch_id=b"status")
    ok = lk_process_while(lk,watch=b"status",watch_finish=True)
    print("ok",ok)
    return status

info = lk_info(lk)
new_recv = []
q = lk_query(baseq)
lk_query_parse(q,"""
prefix:=:/msg
i_index:=:[u32:0]
""")
lk_watch(lk,q,lambda pkt: new_recv.append(pkt))

class Linkmail(cmd.Cmd):
    intro = f"Linkmail - A simple message system ({info.path})"
    prompt = "> "
    not_found=[]
    links=[]
    queue_in= new_recv
    shown=[]
    unchecked = collections.Counter([])
    # last from list, or last 'show'
    last_shown= lk_linkpoint(data="Nothing Shown",create=int(0).to_bytes(8))

    def precmd(self,line):
        lk_process(lk)
        self.unchecked = collections.Counter(map(lambda p: p.spath,self.queue_in))
        return line

    def postcmd(self,stop,_line):
        for k,v in self.unchecked.items():
            print(lk_encode(k,"sp")[2:-1],v)
        return stop

    def do_clear(self,_):
        self.queue_in.clear()
        pass

    def do_status(self,_):
        ex_status = get_exchange_status()
        if not ex_status:
            print("No exchanges active!")
        for e in ex_status:
            print(str(e))

    def do_new(self,path = ""):
        """write a new msg"""
        path_b = lk_eval(f"[/~/msg/{path}]")
        (links,data) = get_input(self.links)
        pkt = lk_keypoint(data=data,links=links,path=path_b)
        print(lk_eval2str(print_template,pkt))
        if (input("Ok[Y/n]?") or "Y") in "Yy":
            lk_save(lk,pkt)
            self.links=[]

    def do_list(self,path = ""):
        """list all messages [path]"""
        path_b = lk_eval(f"[/~/msg/{path}]")
        q = lk_query(baseq)
        lk_query_push(q,"path","=",path_b)
        self.lst = []
        lk_get_all(lk,q,lambda pkt: self.lst.append(pkt))
        for count, msg in enumerate(self.lst):
            print(count, lk_eval2str(list_template,msg))
            self.last_shown = msg

    def do_show(self,idx):
        pkt = self.lst[int(idx)].links if idx else self.last_shown
        self.last_shown = pkt
        print(lk_eval2str(print_template,pkt))

    def do_follow(self,idx):
        """list all links from [idx] pkt"""
        tmp = []
        links = self.lst[int(idx)].links if idx else self.last_shown.links
        for link in links:
            pkt = lk_get(lk,lk_hash_query(link.ptr))
            if not pkt:
                self.not_found = link.ptr
            else:
                tmp.append(pkt)
        for count,(link,pkt) in enumerate(zip(links,tmp)):
            print(count,tag_str(link.tag),lk_eval2str(list_template,pkt))
            self.last_shown = pkt
        self.lst = tmp

    def do_link(self,arg):
        """save a link"""
        if not arg:
            return print(links_str(self.links))
        if arg == "clear":
            self.links = []
            return
        select =  shlex.split(arg)
        link = self.lst[int(select[0])].hash
        tag = select[1] if len(select) > 1 else "link"
        self.links.append(Link(tag,link))
        print(links_str(self.links))

    def do_EOF(self, line):
        return True

dloop = Linkmail()
while True:
    try:
        dloop.cmdloop()
    except Exception as e:
        raise e
    

