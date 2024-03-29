#!/bin/env python3

# This is a bare bone mail TUI to write each other messages with your standard $EDITOR.

import os,sys,logging,tempfile,subprocess,shlex,functools,cmd,argparse,getpass
from typing import Tuple,List
from linkspace import *
from pathlib import Path

#logging.basicConfig(stream=sys.stderr, level=logging.DEBUG)
parser = argparse.ArgumentParser(description='Linkmail')

parser.add_argument('--dir', dest='dir', type=str,
                    default="",help='location of the linkspace instance (default: $LK_DIR | $HOME/linkspace)')
parser.add_argument('--group', dest='group', type=str,
                    default=os.environ.get('LK_GROUP','[#:pub]'),help='group (default: $LK_GROUP | [#:pub])')
parser.add_argument('--key', dest='key', type=str,
                    default=os.environ.get('LK_KEYNAME',"me:local"),help='use key (default: $LK_KEYNAME | me:local)')
parser.add_argument('--password', dest='password',
                    default=os.environ.get('LK_PASS'),help='use key (defaults: $LK_PASS )')

args = parser.parse_args()
lk = lk_open(dir=args.dir)
print(lk_info(lk).dir)

group=lk_eval(args.group)
try:
    print("Using key:",lk_eval2str(f"[@:{args.key}/?b]"))
except:
    print(args.key , " not found - we'll try creating it")
args.password = args.password if args.password is not None else getpass.getpass(prompt='Password> ', stream=None)
print("Unlocking key", args.password);
key = lk_key(lk,password=lk_eval(args.password),name=args.key,create=True)
lk_process(lk) # required for lk_encode to pick up name if lk_key just generated
args.key = lk_encode(key.pubkey,"@/b") # ensure we use the preferred name for this key
print(f"Key ok: ",args.key);

common_q = lk_query_parse(lk_query(),"domain:=:linkmail","group:=:"+args.group)
linkmail_keypoint = functools.partial(lk_keypoint,key=key,domain=b"linkmail",group=group)
linkmail_linkpoint = functools.partial(lk_linkpoint,domain=b"linkmail",group=group)


def tag_str(t):
    """strip nulls from 16 byte value and escape"""
    return lk_eval2str("[0/?a0]",argv=[t])

def links_str(links=[]):
    return "\n".join([tag_str(l.tag) + " " + lk_encode(l.ptr,"#/@/b") for l in links])

first_mail = "Hello world!"
def user_write_mail(links = [],notes = "") -> Tuple[str,List[Link],str]:
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
    lk_status_watch(lk,qid=b"status",
               callback=lambda pkt: status.append(pkt) ,
               timeout=lk_eval("[us:+2s]"),
               domain=b"exchange",
               group=group,
               objtype=b"process")
    ok = lk_process_while(lk,qid=b"status")
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

Use 'help list' for more options
"""


list_template  = "[/or:[/?:[pubkey]/@]:\\[[pubkey/2mini]\\]]:[spacename:str] = [data/?a/slice::20/rpad:20: ] [create/us:delta/rfixed:18: ]:[hash/2mini] # [links_len:str]([:[/links:[tag:str] [ptr/2mini],]/~rcut:32])"
class Linkmail(cmd.Cmd):
    intro =intro
    prompt = "> "

    links : List[Link] = []
    notes = "Add notes here"

    lst : List[Pkt] = []
    queue_in : List[Pkt] = []
    # last from lst, or last 'open'
    last_shown : Pkt = linkmail_linkpoint(data="Nothing here so far",create=int(0).to_bytes(8,byteorder='big'))

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
    
    def print_list(self,lst:List[Pkt]):
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
            print(f"No exchange running for {args.group} - pull requests will be ignored")
            return 
        print(f"Exchange for {args.group} ok:")
        for e in ex_status:
            print(lk_eval2str("([hash/2mini]) [comp2]/[comp3]\\n[data]\\n",e))

    def do_pull(self,spacename= "",):
        """Notify the exchange process to start pulling messages (from [spacename])"""
        ex_status = get_exchange_status()
        if not ex_status:
            print(f"No exchange running for {args.group} - pull requests will be ignored")
            return
        print(f"Pulling from {args.group}")
        logging.debug(ex_status)

        
        q = lk_query(common_q)
        spacename_b = lk_eval(f"[/~/mail/{spacename}]")
        # we use the path in binary form. Two strings might differ but eval to the same bytes
        q = lk_query_push(q,"","qid",spacename_b) 

        q = lk_query_push(q,"spacename","=",spacename_b)
        q = lk_query_push(q,"","follow",b"")
        lk_pull(lk,q)
        
    def do_new(self,spacename):
        """write a new mail"""
        spacename,*rest =  shlex.split(spacename or "/")
        spacename_b = lk_eval(f"[/~/mail/{spacename}]")
        (data,links,notes) = user_write_mail(self.links,self.notes)
        self.notes = notes
        pkt = linkmail_keypoint(data=data,links=links,spacename=spacename_b)
        self.last_shown = pkt
        print(str(pkt))
        if (input("Ok[Y/n]?") or "Y") in "Yy":
            if not get_exchange_status():
                if not ( input(f"No exchange running for {args.group} - Write anyways? [Y/n]") or "Y" in "Yy" ):
                    return
            lk_save(lk,pkt)
            self.links=[]

    def do_threads(self,spacename):
        """List all threads"""
        spacename,*rest =  shlex.split(spacename or "/")
        q = lk_query(common_q)
        q = lk_query_parse(q,f"prefix:=:[/~/mail/{spacename}]","i_branch:=:[u32:0]",*rest)
        logging.debug(q)
        lst = []
        lk_get_all(lk,q,lambda pkt: lst.append(pkt))
        self.print_list(lst)
 
    def do_list(self,spacename):
        """list all messages [spacename] [limit] - e.g. list / recv:>:[now:-1D] pubkey:=:[@:alice:nl] create:>:[-2D]"""
        spacename,*rest =  shlex.split(spacename or "/")
        q = lk_query(common_q)
        q = lk_query_parse(q,f"spacename:=:[/~/mail/{spacename}]",*rest)
        logging.debug(q)
        lst = []
        lk_get_all(lk,q,lambda pkt: lst.append(pkt))
        lst.sort(key = lambda pkt: pkt.create)
        self.print_list(lst)

    def do_open(self,idx):
        pkt = self.lst[int(idx)] if idx else self.last_shown
        self.last_shown = pkt
        print_template= "==[hash:str]==\\n[/~?:[pubkey]/@/b]\\n[spacename:str]\\n[create/us:str]\\n[data/~utf8]\\n"
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

new_mail = lk_query_parse(lk_query(common_q),":qid:incoming","prefix:=:/mail","i_db:<:[u32:0]")
lk_watch(lk,new_mail,lambda p: dloop.new_mail_pkt(p))

dloop.do_pull("")

while True:
    try:
        dloop.cmdloop()
    except Exception as e:
        logging.warning(e, exc_info=True)

