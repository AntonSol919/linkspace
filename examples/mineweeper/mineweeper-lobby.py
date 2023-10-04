#!/bin/env python3

from dataclasses import dataclass,field
from typing import Optional,Dict,List,Tuple # compatibility: python3.8 does not support newer type hints.
import json
import os,functools,logging,sys
logging.basicConfig(stream=sys.stderr, level=logging.DEBUG)

from linkspace import *

group_name = os.environ.get("LK_GROUP","[#:test]")
group = lk_eval(group_name)

lk = lk_open()
key = lk_key(lk)

common_q = lk_query_parse(lk_query(),"domain:=:mineweeper","group:=:"+group_name)
keypoint = functools.partial(lk_keypoint,key=key,domain=b"mineweeper",group=group)

recent_ok = lk_status_watch(
    lk,qid=b"status",callback=lambda _ : True,timeout=lk_eval("[us:+2s]"),
    domain=b"exchange",
    group=group,
    objtype=b"process")

if not recent_ok and lk_process_while(lk,qid=b"status") == 0:
    exit("No exchange process active. ")

@dataclass
class Lobby:
    # empty keypoint at /host/NAME (signed by host)
    create_pkt: Optional[Pkt] = None
    # keypoint at /host/NAME  with a link to create + the game config start data
    start_pkt : Optional[Pkt] = None
    # keypoints(signed by player) at /lobby/[create.hash] Data is player name
    # (leaving is done by overwriting the keypoint without data
    players:Dict[bytes,Pkt] = field(default_factory=dict)
    # keypoints at /lobby/[create.hash]/chat to message each other. 
    chat:List[Pkt] = field(default_factory=list)

print("Welcome to the mineweeper lobby. Pick a game or setup a new one.")
print("Press enter to refresh or enter 'w' to await for change")

# Because we'll be using 'input()' we dont't have asynchronous feedback.
# In a better program we would use a thread for the user interface.

get_lobbies = lk_query_parse(common_q,"prefix:=:/host","depth:=:[u8:2]","create:>:[now:-10m]",":qid:lobbies")
lk_pull(lk,lk_query_parse(get_lobbies,":follow"))

host_lobby = None
lobbies:Dict[Tuple[bytes,bytes],Lobby] = dict()
def insert(p:Pkt):
    lobby = lobbies.get((p.comp1,p.pubkey),Lobby())
    if not len(p.links):
        lobby.create_pkt = p
    else:
        lobby.start_pkt = p
    lobbies[(p.comp1,p.pubkey)] = lobby


lk_watch(lk,get_lobbies,insert)
lk_process(lk);

while host_lobby is None or host_lobby.create_pkt is None:
    try:
        opts = [((b"New Game",key.pubkey),Lobby())] + [l for l in lobbies.items() if l[1].start_pkt is None]
        for i,((name,pubkey),lobby) in enumerate(opts):
            print(f"{i})",name.decode("utf-8"),lk_encode(key.pubkey,"@/b"))
        i = input("<int> to join > ");

        if i == "w":
            lk_process_while(lk,qid=b"lobbies",timeout=lk_eval("[us:+10s]"))
            continue
        if i == "":
            lk_process(lk)
            continue
        i = int(i)
        if i == 0:
            game_name = ""
            while not game_name:
                game_name = input("Game name > ")
            host_space = lk_eval("[//host/[0]]",argv=[game_name])
            create = keypoint(space=host_space)
            lk_save(lk,create)
            lk_process(lk)
            host_lobby = lobbies[(create.comp1,create.pubkey)]
        else:
            lobby = opts[i]
            host_lobby = lobby[1]

    except Exception as e :
        print(e)

print("Using lobby",host_lobby.create_pkt.comp1.decode("utf-8"), " designated admin: ",lk_encode(host_lobby.create_pkt.pubkey,"@/b"))

player_name = ""
while not player_name:
    player_name = input("Player name > ")

get_lobby = lk_query_parse(
    common_q,"prefix:=:/lobby/[hash]",":qid:lobby[hash]",
    pkt=host_lobby.create_pkt)
# we could have used lk_query_push(common_q,"","qid",b"lobby" + host.create_pkt.hash)

def lobby_pkt(p:Pkt):
    global host_lobby
    if p.comp2 == b"chat":
        host_lobby.chat.append(p)
    elif p.comp2 == b"":
        latest = host_lobby.players.get(p.pubkey)
        if latest and latest.create > p.create:
            return
        host_lobby.players[p.pubkey] = p
    else:
        print("unknown msg",p)

lobby_space = lk_eval("[//lobby/[hash]]",pkt=host_lobby.create_pkt)
join_pkt = keypoint(space=lobby_space,data=player_name)
# Its important to understand we used the hash bytes directly.
# There is no encoding to b64.
# host.create_pkt.hash == join_pkt.comp1 and b64(host.create_pkt.hash) == b64(join_pkt.comp1)


lk_save(lk,join_pkt)
lk_watch(lk,get_lobby, on_match=lobby_pkt)
lk_process(lk) # Process ourselves

lk_pull(lk,get_lobby) # request more

me_host = host_lobby.create_pkt.pubkey == key.pubkey
prompt = "[chat msg] | /leave"
if me_host:
    prompt += " | /start"
elif len(host_lobby.players) < 2: # we should wait for info on the existing players
    lk_process_while(lk,qid=b"lobby"+host_lobby.create_pkt.hash)


while not host_lobby.start_pkt:
    # Print the chat log
    print(*[ p.data.decode("utf-8") for p in host_lobby.chat],sep="\n",end="\n")
    # Print the player names
    print("Players:", ",".join([ p.data.decode("utf-8") for p in host_lobby.players.values() if p.data]))
    print(prompt)
    try:
        cmd = input("> ")
    except KeyboardInterrupt:
        cmd = "/leave"
    if cmd == "":
        #lk_process_while(lk,timeout=lk_eval("[us:+2s]"))
        lk_process(lk)
    elif me_host and cmd == "/start":
        lk_process(lk)
        players = [ [p.data.decode("utf-8"),b64(p.pubkey)] for p in host_lobby.players.values() if p.data]
        print(*players,sep="\n")
        try:
            cols = int(input("cols (10) > ") or "10")
            rows = int(input("rows (10) > ") or "10")
            mine_rate = float(input("mine_rate (0.2) > ")or "0.2")
            data = json.dumps({"columns":cols,"rows":rows,"mine_rate":mine_rate,"players":players})
            start_pkt = keypoint(spacename=host_lobby.create_pkt.spacename,data=data,links=[Link("create",host_lobby.create_pkt.hash)])
            lk_save(lk,start_pkt)
            lk_process(lk)
        except Exception as e:
            print(e)

    elif cmd == "/leave":
        lk_save(lk,keypoint(space=lobby_space))
        if me_host:
            leave_pkt = keypoint(spacename=host_lobby.create_pkt.spacename,links=[Link("create",host_lobby.create_pkt.hash)]) # no data means we closed the lobby.
            lk_save(lk,leave_pkt)
            print("host left ",leave_pkt)
        lk_process(lk)
        exit("We left.")
    else:
        lk_save(lk,keypoint(space=lobby_space + lk_eval("[//chat]"),data=cmd))
        lk_process(lk)

if not len(host_lobby.start_pkt.data):
    exit("Host left.")

print("Starting game", host_lobby.start_pkt, host_lobby.start_pkt.data.decode("utf-8"))

# A real program would import "mineweeper-multiplayer" and just call a function to start the game.
# For the purpose of the tutorial, we keep the two scripts loosely coupled.
srcpath = os.path.dirname(os.path.realpath(__file__))
game_setup_hash = b64(host_lobby.start_pkt.hash)
os.system(f"python \"{srcpath}/mineweeper-multiplayer.py\" {game_setup_hash}")
