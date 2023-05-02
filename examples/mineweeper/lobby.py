#!/bin/env python3

from dataclasses import dataclass,field
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

recent_ok = lk_status_poll(
    lk,qid=b"status",callback=lambda _ : True,timeout=lk_eval("[s:+2s]"),
    domain=b"exchange",
    group=group,
    objtype=b"process")

if not recent_ok and lk_process_while(lk,qid=b"status") == 0:
    exit("No exchange process active. ")

@dataclass
class Lobby:
    # empty keypoint at /host/NAME (signed by host)
    create_pkt: Pkt | None= None
    # keypoint at /host/NAME  with a link to create + the game config start data
    start_pkt : Pkt | None= None
    # keypoints(signed by player) at /lobby/[create.hash] Data is player name
    # (leaving is done by overwriting the keypoint without data
    players:dict[bytes,Pkt] = field(default_factory=dict)
    # keypoints at /lobby/[create.hash]/chat to message each other. 
    chat:list[Pkt] = field(default_factory=list)


get_lobbies = lk_query_parse(common_q,"prefix:=:/host","path_len:=:[u8:2]","create:>:[now:-10m]",":qid:lobbies")
lk_pull(lk,lk_query_parse(get_lobbies,":follow"))

host = None
lobbies:dict[tuple[bytes,bytes],Lobby] = dict()
def insert(p:Pkt):
    print("found p",p)
    lobby = lobbies.get((p.path1,p.pubkey),Lobby())
    if not len(p.links):
        lobby.create_pkt = p
    else:
        lobby.start_pkt = p
    lobbies[(p.path1,p.pubkey)] = lobby


lk_watch(lk,get_lobbies,insert)
lk_process(lk);
while host is None or host.create_pkt is None:
    try:
        print(lobbies)
        opts = [(("New Game",key.pubkey),Lobby())] + [l for l in lobbies.items() if l[1].start_pkt is None]
        for i,((name,pubkey),lobby) in enumerate(opts):
            print(i,name,lk_encode(key.pubkey,"@/b"))
        i = input("<int> to join, empty to refresh (w to wait) > ");

        if i == "w":
            lk_process_while(lk,qid=b"lobbies",timeout=lk_eval("[s:+10s]"))
            continue
        if i == "":
            lk_process(lk)
            continue
        i = int(i)
        if i == 0:
            game_name = input("game name > ")
            host_path = lk_eval("[//host/[0]]",argv=[game_name])
            create = keypoint(path=host_path)
            lk_save(lk,create)
            host = Lobby(create_pkt = create)
            lk_process(lk)
        else:
            lobby = opts[i]
            host = lobby[1]

    except Exception as e :
        print(e)

print("Using lobby",host.create_pkt.path1.decode("utf-8"), " designated admin: ",lk_encode(host.create_pkt.pubkey,"@/b"))

my_name = input("Using name?")
lobby_path = lk_eval("[//lobby/[hash]]",pkt=host.create_pkt)
join_pkt = keypoint(path=lobby_path,data=my_name)
lk_save(lk,join_pkt)

get_lobby = lk_query_parse(
    common_q,"prefix:=:/lobby/[hash]",":qid:lobby[hash]",
    pkt=host.create_pkt)

def lobby_pkt(p:Pkt):
    global host
    if p.path2 == b"chat":
        host.chat.append(p)
    elif p.path2 == b"":
        latest = host.players.get(p.pubkey)
        if latest and latest.create > p.create:
            return
        host.players[p.pubkey] = p
    else:
        print("unknown msg",p)

lk_pull(lk,get_lobby)
lk_watch(lk,get_lobby, on_match=lobby_pkt)

me_host = host.create_pkt.pubkey == key.pubkey
prompt = "[chat msg] | /leave"
if me_host:
    prompt += " | /start"

while not host.start_pkt:
    print(*[ p.data.decode("utf-8") for p in host.chat],sep="\n")
    print(*[ p.data for p in host.players.values() if p.data],sep=" ")
    cmd = input(prompt)
    if cmd == "":
        lk_process_while(lk,timeout=lk_eval("[s:+2s]"))
    if me_host and cmd == "/start":
        players = [ [p.data.decode("utf-8"),b64(p.pubkey)] for p in host.players.values() if p.data]
        try:
            cols = int(input("cols"))
            rows = int(input("rows"))
            mine_rate = float(input("mine_rate"))
            data = json.dumps({"columns":cols,"rows":rows,"mine_rate":mine_rate,"players":players})
            start_pkt = keypoint(path=lobby_path,data=data,links=[Link("create",host.create_pkt.hash)])
            lk_save(lk,start_pkt)
            lk_process(lk)
        except Exception as e:
            print(e)

    if cmd == "/leave":
        lk_save(lk,keypoint(path=lobby_path))
        exit("left")
    else:
        lk_save(lk,keypoint(path=lobby_path + lk_eval("[//chat]"),data=cmd))
        lk_process(lk)

print("Starting game", host.start_pkt, host.start_pkt.data.decode("utf-8"))
