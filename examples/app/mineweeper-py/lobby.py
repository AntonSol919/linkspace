#!/bin/env python3

import os,getpass,functools,cmd,traceback,logging,sys
logging.basicConfig(stream=sys.stderr, level=logging.DEBUG)

from mineweeper import MineWeeper, clear_screen
from linkspace import *
import re

lkdir = os.environ.get("LK_DIR","")
group_expr = os.environ.get("LK_GROUP","[#:test]")
group = lk_eval(group_expr)
key = os.environ.get("LK_KEYNAME","me:local")
password = os.environ.get("LK_PASS","")

lk = lk_open(dir=lkdir,create=True)
key = lk_key(lk,password=lk_eval(password),name=key,create=True)
# key = lk_keygen()

common_q = lk_query_parse(lk_query(),"domain:=:mineweeper","group:=:"+group_expr)
lk_keypoint = functools.partial(lk_keypoint,key=key,domain=b"mineweeper",group=group)
lk_linkpoint = functools.partial(lk_linkpoint,domain=b"mineweeper",group=group)

ls = []
recent = lk_status_poll(lk,qid=b"status",callback=lambda x : ls.append(x),
               timeout=lk_eval("[s:+2s]"),
               domain=b"exchange",
               group=group,
               objtype=b"process")
proc_watch = lk_process_while(lk,qid=b"status")
if not recent and proc_watch == 0:
    exit("No exchange process active. ")

lobbies_query = lk_query_parse(common_q,"prefix:=:/lobby","path_len:=:[u8:3]","create:>:[now:-10m]",":qid:get_lobby","i_branch:=:[u32:0]")
lk_pull(lk,lk_query_parse(lobbies_query,":follow"))
lk_process(lk)


class SetupLobby():
    game_start_pkt : Pkt | None
    host : Pkt # public key
    lobby_query : Query
    lobbies = dict()
    players = set()
    stage = "pick_game"
    new_packets = []
    my_name = input("Using name > ")

    def refresh(self):
        print(self.new_packets)
        self.new_packets.clear()
        getattr(self, self.stage)()

    def pick_game(self):
        self.lobbies = dict()
        def insert(p):
            lobby = self.lobbies.get((p.comp1,p.comp2),set())
            lobby.update(p.links)
            self.lobbies[p.comp1] = lobby

        lk_get_all(lk,lobbies_query,insert)
        opts = list(self.lobbies.items())
        opts.append(("New Game",set()))
        for i,lobby in enumerate(opts):
            print(i,lobby[0],len(lobby[1]))
        try:
           i = input("<int> to join, empty to refresh > ");
           if i == "":
               print("Awaiting next")
               return
           i = int(i)
           if i == len(opts)-1:
               lobby_name = input("game name > ")
               lobby_path = lk_eval("[//lobby/[0]/[1]]",argv=[lobby_name,key.pubkey])
               print("Save ok")
               self.host = lk_keypoint(path=lobby_path,links=[Link(self.my_name,key.pubkey)])
               lk_save(lk,self.host)
               self.lobby_query = lk_query_parse(lobbies_query,"i_branch:=:[u32:0]","path:=:[spath]",pkt=self.host)
               self.stage = "host_await_players"
           elif i >= 0 and i < len(opts):
               
               print("todo")
        except Exception as e :
            logging.exception(e)
            return

    def player_await_host(self):
        q = lk_query_parse(common_q,
                           "path:=:[//lobby/[comp1]/[pubkey]/start]",
                           "pubkey:=:[pubkey]",
                           "create:>:[create]",
                           "i:=:[u32:0]",
                           ":qid:start_game",
                           pkt=self.host)
        go = []
        lk_watch(lk,q,callback= lambda x:go.append(x))
        lk_process_while(lk,qid=b"start_game")
        if not go:

            return
        for link in go[0].links:
            if link.ptr == key.pubkey:
                self.game_start_pkt = go[0]
                return 
        print("Game started without you")
        self.stage ="pick_game"



    def host_await_players(self):
        self.players = set(self.host)
        lk_get_all(lk,self.lobby_query,lambda p: self.players.update(p.links) if self.host in p.links else None)
        print(self.players)
        i = input("Start or empty to wait")
        if i == "":
            return
        if i.lower()[0] == 's':
            self.game_start_pkt = lk_keypoint(path)

setup = SetupLobby()

setup.refresh()
lk_watch(lk,lk_query_push(lobbies_query,"i_db","<",int(0).to_bytes(4)),lambda p: setup.new_packets.append(p))

while True:
    lk_process_while(lk,qid=b"get_lobby",timeout=lk_eval("[s:+2s]"))
    lk_process(lk)
    setup.refresh()
