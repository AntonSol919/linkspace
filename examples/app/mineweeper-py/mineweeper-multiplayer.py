#!/bin/env python3

import sys, json,re,functools

from mineweeper import MineWeeper, clear_screen
from linkspace import *

# The players have agreed on the game setup and players.
# The game setup was encoded in a linkspace.Pkt we agreed upon through some other system (like a game lobby)
# The argument to this script is the hash of this Pkt. 
game_hash = sys.argv[1] 

# Open the default instance and the default signing key
lk = lk_open() 
signing_key = lk_key(lk)

# Get the game setup packet.
game_pkt = lk_get(lk, lk_hash_query(game_hash)) or exit("No such game start")
"""
The data of this game pkt is a json encoding similar to:
{
    "rows":20,
    "columns": 20,
    "mine_rate":0.3,
    "players":[
        ["alice", "iyfplTMDFj3Jw8XwdfwvXs9ZVgwwYZNsQ3E7cS55kLQ"],
        ["bob", "qfurOQ2oTD1Xc9dQf_gX2MXbsbALdXQ_XemF0aTlj6U"],
        ["charlie", "2SYK3NlS8k4ELKWR6CmqIQAiPrMosr5LioK7456jnDY"]
    ]
}
"""
game_conf = json.loads(game_pkt.data.decode("utf-8"))

print("Starting game")
# To notify the user about the group used, lets print the group name corresponding to the bytes in the game_pkt linkpoint
# lk_encode takes bytes as an argument and tries various ways to print them in abe format.
print("group  :", lk_encode(game_pkt.group,"#/@/b"))
print(game_conf)

# The players are a subset of the users in a group.
# Not every user is interested in every packet.
# We have to make it obvious which packets are part of this game, so the players can query it and the others can ignore it.
# We'll use this unique path to do so.
game_path = path([b"game", game_pkt.hash])

# We'll use slightly different queries throughout this process, but much of their predicates remain static.
# This query will act as our template for more specific queries. 
common_q = lk_query_parse(lk_query(),"domain:=:mineweeper","group:=:[group]","path:=:[0]", pkt=game_pkt, argv=[game_path])

# The [group] and [0] resolved to the bytes from the pkt and argv arguments respectively.
# print(lk_query_print(common_q, True))

# The first use of our query is to signal the linkspace instance that we're interested in packets related to this game.
# This signaling is done by writing the query to [f:exchange]:[#:0]:/[q.group]/[q.domain]/[q.qid] 
# The lk_pull function encodes this convention and does some checking.  
pull_q = lk_query_push(common_q,"","qid", b"game"+game_pkt.hash)
lk_pull(lk, pull_q)

# An exchange process reads this request.  
# If everything is going to plan, all players will eventually receive packets saved by others that matches the query.

# A packet can be one of three types. ( Or to be exact, a packet is a [netheader,hash,point] and there are three types of points)
# A datapoint, linkpoint, or keypoint.
# A datapoint only holds data. 
# A linkpoint holds data and other fields including: links to other packets, and a path to query it by.
# A keypoint is a linkpoint with a signature. 
# For our case we'll only use keypoints. 

# Its common to wrap the *point functions as we know the following arguments wont change.
new_keypoint = functools.partial(lk_keypoint, key=signing_key, domain=b"mineweeper", group=game_pkt.group, path=game_path)

player_count = len(game_conf['players'])
for i, e in enumerate(game_conf['players']):
    print(i, e)

players = list([e[0] for e in game_conf['players']])


# By setting the seed to be the game_pkt.hash, every player will generate the same map of mines.
game = MineWeeper(    players=players,
    rows=game_conf['rows'],
    cols=game_conf['columns'],
    mine_rate=game_conf['mine_rate'],
    seed=game_pkt.hash)
input("Coordinates are row/vertical, col/horizontal.")

# We expect the players to take turns in sequence until someone reveals a bomb.
# Alice -> Bob -> Charlie -> Alice -> Bob -> ...

# Each turn will be a keypoint, signed by the player, with the data being a json encoded [x,y] for the cell the player revealed.
# By checking the pubkey of the Pkt we know who made the move, but we'll need a way to know which turn we're on.
# One option is to add a {turn:int} field to the json.
# We can be more strict and require each keypoint to link back to the prev turn.
# To start, the first turn will link back to the game_pkt
prev_turn = Link("prev", game_pkt.hash)

# We'll use a callback that receives packets matching (a subset of) our query. 
# If the pkt argument has a link equal to prev_ptr it contains the json for the next turn. 
def find_and_do_next_move(player_turn_pkt):
    global prev_turn, game
    if player_turn_pkt.links[0] == prev_turn:
        try:
            [row, col] = json.loads(player_turn_pkt.data.decode("utf-8"))
            clear_screen()
            game.reveal(row, col)
            # Update our prev_ptr to the this packet. 
            prev_turn = Link("prev", player_turn_pkt.hash)
            return True # stop looking
        except Exception as e :
            exit("Game was corrupted: " + str(e) )

while game.print_game_state():
    [pid,name] = game.current_player();
    [_name,b64_key] = game_conf['players'][pid]

    # We know whose turn it is, so we'll narrow our query down to packets signed by the current player.
    # We have the pubkey in base64 string encoding so we'll use lk_query_parse instead of lk_query_push
    q = lk_query_parse(common_q,"pubkey:=:"+b64_key)
    # (We don't have to lk_pull because it's a subset of what we're already pulling).

    # To start off, we'll check if any of the packets we have locally represent the turn we're looking for.
    # (As a side effect, we can close and open this script and continue were we left off)
    # lk_get_all returns a positive number if the callback was finished by returning True.
    if lk_get_all(lk,q,find_and_do_next_move) > 0:
        continue

    # We don't have the packet for the current turn. 
    # If the current players key is our key, we should make a move.
    if b64_key == b64(signing_key.pubkey):
        print("Your turn")
        try:
            [row,col] = re.split('[;|, :]',input("Reveal:"))
            data=json.dumps([int(row),int(col)])
            pkt = new_keypoint(data=data,links=[prev_turn])
            print("Cheating ",game.is_mine(int(row),int(col)))
            if "y" in input("?"):
                # We save the packet. An exchange process will ensure the other players get it. 
                lk_save(lk,pkt)
                prev_turn = Link("prev",pkt.hash)
            # 'input("?")' might have taken the user a long time, 
            # and saving a packet doesn't update our local view of the database. 
            # To do so we have to process the new packets.
            # In our case that doesn't do anything else.  
            lk_process(lk)
        except Exception as e:
            print(e)
    else:
        # If its not our turn, we'll have to wait for the packet of the current player to arrive. 
        print("Waiting")

        # lk_watch is like lk_get_all, but in addition the callback will also get called for matches during lk_process or lk_process_while.

        # lk_watch requires we give our query a qid.
        # This allows us to remove or overwrite it later.
        # (in our case the cb will finish by returning True, so this isn't very relevant except for logging/debugging).
        # We'll name our query 'move' 
        q = lk_query_push(q,"","qid",b"move") # alternatively lk_query_parse(q,":qid:move")

        # lk_watch, just like lk_get_all, starts with the database.
        # We can skip this. We already used lk_get_all (and we haven't lk_process* since).
        # To skip the db phase we add a predicate that i_db should be < 0 
        q = lk_query_push(q,"i_db","<",int(0).to_bytes(4))

        # If we had not set i_db:<:[u32:0] , lk_watch would check the database before saving the callback and returning. 
        lk_watch(lk,q,find_and_do_next_move)

        # lk_process_while is similar to lk_process but can wait for new packets until a condition is met.
        # Linkspace processes can get complex registering many callbacks with lk_watch and waiting for multiple unordered packets. 
        # Our case is simple.
        # There is only one watch, and only one reason for it to be done.
        lk_process_while(lk,qid=b"move")
        # The current player made a move, and we can continue to the next round

# Our game is done
print("Fin")
