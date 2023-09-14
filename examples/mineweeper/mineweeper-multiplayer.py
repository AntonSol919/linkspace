#!/bin/env python3

import sys, json,re,functools

from mineweeper import MineWeeper, clear_screen
from linkspace import *

# The players agreed on the game setup and players.
# This was saved in a packet.
# This script is launched with that Pkt.hash in base64.

game_hash = sys.argv[1]

# Open the linkspace instance and our key. (Set with LK_DIR and LK_KEYNAME)

lk = lk_open() 
signing_key = lk_key(lk)

# Get the game setup packet.
game_pkt = lk_get(lk, lk_hash_query(game_hash)) or exit("No such game start")

"""
We have decided the data will be json encoded and look like:
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

# We print some game info.
print("Starting game")
# lk_encode takes bytes and tries various ways to print them in readable abe format.
print("group  :", lk_encode(game_pkt.group,"#/@/b"))
print(game_conf)

# The players are a subset of the members in a group.
# They must signal what data they want from the group.
# We make it unambiguous which packets are part of this game by reading/writing to a unique spacename.
game_spacename = space([b"game", game_pkt.hash])
# Internally, spacenames are length separated bytes. Most arguments accept a string like /game/[b:AAAAA....] .
# In our case we can skip this b64 encoding step.

# Every query has at least the following in common.
common_q = lk_query_parse(
    lk_query(),
    "domain:=:mineweeper",
    "group:=:[group]",
    "spacename:=:[0]",
    pkt=game_pkt, argv=[game_spacename]) #provides the [group] and [0] values

# To inspect a query use print(lk_query_print(common_q, True))

# We signal the group to send us packets by saving the query in a packet to a specific location.
# Instead of doing so manually, we use lk_pull.
# pulling requires the query to have a qid so we can remove it later.
pull_q = lk_query_push(common_q,"","qid", b"game"+game_pkt.hash)
lk_pull(lk, pull_q)

# If everything is going to plan:
# An exchange process reads the request, and ensure all players eventually receive packets from others that matches the query.
# i.e. whenever we save a point with the spacename 'game_spacename', the other players who ran lk_pull receive that packet.

# A packet is one of three types. (Or to be exact, a packet is a [netheader,hash,point] and there are three types of points)
# A datapoint, linkpoint, or keypoint.
# A datapoint holds: data.
# A linkpoint holds: data, a spacename, and links (tag,hash) to other packets.
# A keypoint is a linkpoint with a cryptographic signature.
# For our case we'll only use keypoints.

# Its common to wrap the *point functions when arguments wont change.
new_keypoint = functools.partial(lk_keypoint, key=signing_key, domain=b"mineweeper", group=game_pkt.group, spacename=game_spacename)

player_count = len(game_conf['players'])
for i, e in enumerate(game_conf['players']):
    print(i, e)

players = list([e[0] for e in game_conf['players']])


# The pkt hash is used as a seed to generate the mine map.
game = MineWeeper(    players=players,
    rows=game_conf['rows'],
    cols=game_conf['columns'],
    mine_rate=game_conf['mine_rate'],
    seed=game_pkt.hash)
input("Coordinates are row/vertical, col/horizontal.")

# Players take turns until someone reveals a bomb.
# Its up to us as a developer to choose how to encode that.
# We decide to encode a turn in a keypoint, signed by the player, with the data a json encoded [x,y] for the chosen cell.
# The Pkt.pubkey indicates who made the move.
# We could add a {turn:int} field to the json.
# Instead, we chose to be more strict:
# every turn will link to the previous turn.
# This makes it obvious what happened if someone adds multiple moves for some reason.
# The first turn will link back to the game_pkt
prev_turn = Link("prev", game_pkt.hash)

# Packets that match (a subset of) our query are processed as follows:
# If the pkt has its links equal to prev_ptr, and pubkey == current_player.pubkey => the data should contain the json for their turn.
def find_and_do_next_move(player_turn_pkt):
    global prev_turn, game
    if list(player_turn_pkt.links) != [prev_turn]: # We skip the check for current_player.pubkey, it shall be handled through the query.
        return
    [row, col] = json.loads(player_turn_pkt.data.decode("utf-8"))
    clear_screen()
    game.reveal(row, col)
    # Update our prev_ptr to the this packet. 
    prev_turn = Link("prev", player_turn_pkt.hash)
    # Returning True stops this function from being called with more matches.
    return True

while game.print_game_state():
    [pid,name] = game.current_player();
    [_name,player_b64_key] = game_conf['players'][pid]

    # We narrow our query to packets signed by the current player.
    # We have their pubkey in a base64 string.
    # lk_query_push and lk_query_parse can both update the query. The later handles strings formats instead of bytes.
    q = lk_query_parse(common_q,"pubkey:=:"+player_b64_key)

    # (We don't have to lk_pull. This query is a subset of what we're already pulling).

    q = lk_query_push(q,"","qid",b"move") # eqv: lk_query_parse(q,":qid:move").

    # To start check the database for a match.
    # lk_watch first checks the database. If the callback returned True it returns a positive number.
    # Otherwise 0 or negative to indicate the callback was registered and is waiting for new packets.
    if lk_watch(lk,q,find_and_do_next_move)> 0:
        # next turn
        # (As a side effect of our design, a user can close and re-open the game and continue were they left off)
        continue

    # If its our turn, do a move.
    if player_b64_key == b64(signing_key.pubkey):
        while True:
            print("Your turn")
            try:
                [row,col] = re.split('[;|, :]',input("Reveal:"))
                (row,col) = (int(row),int(col))
                data=json.dumps([row,col])
                game.get_revealable_cell(row,col)
                #if "i'm" and not "a cheater" and game.is_mine(row,col) and "c" in input("cheat?"):
                #    continue
                pkt = new_keypoint(data=data,links=[prev_turn])
                # We save the packet. An exchange process will ensure the other players get it.
                # (This does not update our view of the database until we call lk_process or lk_process_while)
                lk_save(lk,pkt)
                break
            except Exception as e: 
                print(e)
    else:
        print("Waiting")

    # We process new packets until our callback registered with qid 'move' returned True.
    # lk_process_while is smart about sleeping until new packets arrive and a condition is met.
    # a positive value means the callback has finished.
    while not lk_process_while(lk,qid=b"move") > 0:
        pass
    # The current player made a move. We continue to the next round

# Our game is done
print("Fin")
