#!/bin/env python3.11
import os,sys,logging,tempfile,subprocess,shlex,functools,cmd,argparse,getpass
from lkpy import *
from pathlib import Path

#logging.basicConfig(stream=sys.stderr, level=logging.DEBUG)
parser = argparse.ArgumentParser(description='Linkmail')
parser.add_argument('--linkspace', dest='linkspace', type=str,
                    default="",help='use linkspace (default: $HOME/linkspace)')
parser.add_argument('--group', dest='group', type=str,
                    default=os.environ.get('GROUP'),help='group (default: [#:pub])')
parser.add_argument('--key', dest='key', type=str,
                    default=os.environ.get('LK_KEY'),help='use key (default: [me:local])')
args = parser.parse_args()

lk = lk_open(path=args.linkspace,create=True)
print(lk_info(lk).path)

keyname ="me:local"
password=""
group_name = args.group
group=lk_eval(group_name)
common_q = lk_query_parse(lk_query(),"domain:=:linkmail","group:=:"+group_name)
try:
    print("Using key:",lk_eval2str(f"[@:{keyname}/?b]"))
except:
    print(keyname , " not found - we'll try creating it")
passw = getpass.getpass(prompt='Password> ', stream=None)
print("Unlocking key");
key = lk_key(lk,password=lk_eval(passw),name=keyname,create=True)
lk_process(lk) # required for lk_encode
keyname = lk_encode(key.pubkey,"@/b") # ensure we use the preferred name for this key
print(f"Key ok: ",keyname);

lk_keypoint = functools.partial(lk_keypoint,key=key,domain=b"linkmail",group=group)
lk_linkpoint = functools.partial(lk_linkpoint,domain=b"linkmail",group=group)



def tag_str(t):
    """strip nulls from 16 byte value and escape"""
    return lk_eval2str("[0/?a0]",argv=[t])

def links_str(links=[]):
    return "\n".join([tag_str(l.tag) + " " + lk_encode(l.ptr,"#/@/b") for l in links])

first_mail = "Hello world!"
def user_write_mail(links = [],notes = "") -> tuple[str,list[Link],str]:
    global first_mail
    editor = os.environ.get('VISUAL', os.environ.get('EDITOR', 'vi'))
    with tempfile.NamedTemporaryFile(mode='w+b', delete=False) as tf:
        tf.write(f"{first_mail}\n==LINKS==\n".encode())
        tf.write(links_str(links).encode())
        if notes:
            tf.write(f"\n==NOTES==\n{notes}".encode())
        tf.flush()
        # will not work on windows - must first close file
        tmp_file = tf.name
        process = subprocess.Popen(shlex.split(f"{editor} \"{tmp_file}\""))
        process.wait()
        first_mail = ""
    mail,*rest = Path(tmp_file).read_text("utf-8").split("\n==LINKS==\n",1)
    if not rest:
        return (mail,[],"")
    links,*notes = rest[0].split("\n==NOTES==\n",1)
    def read_link(line):
        print(line)
        [tag,link] = line.split(maxsplit=1)
        return Link(lk_eval(f"[a:{tag}]"),lk_eval(link))
    links = [read_link(line) for line in links.splitlines() if line]
    return (mail,links,notes[0] if notes else "")

def get_exchange_status(watch_finish=False):
    status =[] 
    lk_status_poll(lk,
               lambda pkt: status.append(pkt) ,
               timeout=lk_eval("[s:+2s]"),
               domain=b"exchange",
               group=group,
               objtype=b"process",watch_id=b"status")
    ok = lk_process_while(lk,watch=b"status",watch_finish=watch_finish)
    return status

intro = """
linkmail - A simple linkspace mail system
Use 'new' to write a new linkmail.
Use 'list [subj] [limit]' to print a list of linkmail.
Use 'pull [subj]' to request mail from the group.
Use 'queue' to print a list of recently received linkmail
Use 'open [N]' to open a linkmail

Every time a list is displayed with a number you can:
- 'open <N>' to open it
- 'link <N> [tag]' to save it for use during 'new'
"""


list_template  = "[/or:[/?:[pubkey]/@]:\\[[pubkey/2mini]\\]]:[path:str] = [data/slice::20/rpad:20: ] [create/s:delta/rfixed:18: ]:[hash/2mini] # [links_len:str]([:[/links:[tag:str] [ptr/2mini],]/~rcut:32])"
class Linkmail(cmd.Cmd):
    intro =intro
    prompt = "> "

    links : list[Link] = []
    notes = "Add notes here"

    lst : list[Pkt] = []
    queue_in : list[Pkt] = []
    # last from lst, or last 'open'
    last_shown : Pkt = lk_linkpoint(data="Nothing here so far",create=int(0).to_bytes(8))

    def precmd(self,line):
        lk_process(lk)
        return line

    def postcmd(self,stop,_line):
        if self.queue_in:
            print("New messages (use 'queue')")
        return stop

    def print_entry(self,pkt:Pkt,prnt=True,tag="") -> int:
        i = len(self.lst)
        self.lst.append(pkt)
        self.last_shown = pkt;
        if prnt:
            print(i,tag,lk_eval2str(list_template,pkt))
        return i
    
    def print_list(self,lst:list[Pkt]):
        self.lst.clear()
        for p in lst:
            self.print_entry(p)

    def do_queue(self,_):
        """List the packets received since the last call to queue (or starting this proc)"""
        self.print_list(self.queue_in)
        self.queue_in.clear()

    def do_status(self,_):
        ex_status = get_exchange_status(True)
        if not ex_status:
            print(f"No exchange running for {group_name} - pull requests will be ignored")
        for e in ex_status:
            print(lk_eval2str("([hash/2mini]) [comp2]/[comp3]\\n[data]\\n",e))

    def do_pull(self,path = "",):
        """Notify the exchange process to start pulling messages (from [path])"""
        if not get_exchange_status():
            print(f"No exchange running for {group_name} - pull requests will be ignored")
            return
        q = lk_query(common_q)
        path_b = lk_eval(f"[/~/mail/{path}]")
        # we use the path in binary form. Two strings might differ but eval to the same bytes
        q = lk_query_push(q,"","watch",path_b) 

        q = lk_query_push(q,"path","=",path_b)
        q = lk_query_push(q,"","follow",b"")
        lk_pull(lk,q)
        
    def do_new(self,path):
        """write a new mail"""
        path,*rest =  shlex.split(path or "/")
        path_b = lk_eval(f"[/~/mail/{path}]")
        (data,links,notes) = user_write_mail(self.links,self.notes)
        self.notes = notes
        pkt = lk_keypoint(data=data,links=links,path=path_b)
        self.last_shown = pkt
        print(str(pkt))
        if (input("Ok[Y/n]?") or "Y") in "Yy":
            if not get_exchange_status():
                if not ( input(f"No exchange running for {group_name} - Write anyways? [Y/n]") or "Y" in "Yy" ):
                    return
            lk_save(lk,pkt)
            self.links=[]

    def do_threads(self,path):
        """List all threads"""
        path,*rest =  shlex.split(path or "/")
        q = lk_query(common_q)
        q = lk_query_parse(q,f"prefix:=:[/~/mail/{path}]","i_branch:=:[u32:0]",*rest)
        logging.debug(q)
        lst = []
        lk_get_all(lk,q,lambda pkt: lst.append(pkt))
        self.print_list(lst)
 
    def do_list(self,path):
        """list all messages [path] [limit] - e.g. list / recv:>:[now:-1D] pubkey:=:[@:alice:nl] create:>:[-2D]"""
        path,*rest =  shlex.split(path or "/")
        q = lk_query(common_q)
        q = lk_query_parse(q,f"path:=:[/~/mail/{path}]",*rest)
        logging.debug(q)
        lst = []
        lk_get_all(lk,q,lambda pkt: lst.append(pkt))
        self.print_list(lst)

    def do_open(self,idx):
        pkt = self.lst[int(idx)] if idx else self.last_shown
        self.last_shown = pkt
        print_template= "==[hash:str]==\\n[/~?:[pubkey]/@/b]\\n[path:str]\\n[create/s:str]\\n[data:str]\\n"
        print(lk_eval2str(print_template,pkt))
        self.lst.clear()
        for link in pkt.links:
            pkt = lk_get(lk,lk_hash_query(link.ptr))
            if not pkt:
                print("\t",tag_str(link.tag), " ", lk_encode(link.ptr,"#/@/b"))
            else:
                print(len(self.lst),"\t",tag_str(link.tag), " ", b64(link.ptr,mini=True),lk_eval2str(list_template,pkt))
                self.lst.append(pkt)

    def do_link(self,arg):
        """save a link used in the next mail"""
        if arg == "clear":
            self.links = []
            return
        select,*rest =  shlex.split(arg or "shown")
        link = self.last_shown.hash if select =="shown" else self.lst[int(select[0])].hash
        tag = rest[0] if rest else "link"
        self.links.append(Link(tag,link))
        print("Current Links")
        print(links_str(self.links))

    def do_EOF(self, _):
        return True

    def new_mail_pkt(self,pkt):
        if pkt.pubkey != key.pubkey:
            self.queue_in.append(pkt)


dloop = Linkmail()

new_mail = lk_query_parse(lk_query(common_q),":watch:incoming","prefix:=:/mail","i_index:<:[u32:0]")
logging.debug("new_mail",str(new_mail))
lk_watch(lk,new_mail,lambda p: dloop.new_mail_pkt(p))

dloop.do_pull("")

while True:
    try:
        dloop.cmdloop()
    except Exception as e:
        logging.warn(e, exc_info=True)
