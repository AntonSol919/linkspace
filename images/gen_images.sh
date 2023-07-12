#!/bin/bash
export LK_PASS=""
ALICE=$(lk key --no-pubkey --no-lk)
BOB=$(lk key --no-pubkey --no-lk)

lk keyp msg_board:[#:test]:/image/BrokenMachine.jpg --enckey $ALICE --data-str "(image data)" > /tmp/msgboard
HASH=($(cat /tmp/msgboard | lk pktf [hash:str]))
lk keyp "msg_board:[#:test]:/thread/thread/Coffee machine broke!/msg" --enckey $ALICE \
   --data-str "Fix pls?" \
   -- "image link:${HASH[0]}">> /tmp/msgboard

HASH=($(cat /tmp/msgboard | lk pktf [hash:str]))
lk keyp "msg_board:[#:test]:/thread/thread/Coffee machine broke!/msg" --enckey $BOB \
   --data-str "Hey [link:reply to]. Isn’t [link:image link] from 2015?" \
   -- "reply to:${HASH[1]}" "image link:${HASH[0]}" >> /tmp/msgboard

{
    echo ' digraph G{ rankdir=RL ; rank="same" ; node[shape="record"] ;' 
    lk p --pkts /tmp/msgboard  '"p[hash:str]"\[label=" { <hash> [hash/2mini] }  | { signed: [pubkey/2mini] | [path:str] } | data=[data] | [links_len:str] links [/links: | <[i:str]> [tag:str]\: [ptr/2mini] ] "\];
     [/links: "p[hash:str]"\:[i:str] -> "p[ptr:str]"\:hash ;\n ]' ;
     echo "}" ;
} | dot -Tsvg > msg_board.svg

