




#!/bin/env python3

import sys, json,re

from mineweeper import MineWeeper, clear_screen
from linkspace import *

# The first argument is the appointed hash
game_id = sys.argv[1] 

# Open the linkspace instance.
lk = lk_open() 
# This will use name=($LK_KEYNAME | me:local) and pass=($LK_PASS | "") by default.
key = lk_key(lk)

# Get the packet and decode the json data
game_pkt = lk_get(lk, lk_hash_query(game_id))
"""
Its expected this is something like:
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
# lk_encode will try and print a pretty name for the group, or fallback to base 64. 
print("group  :", lk_encode(game_pkt.group,"#/@/b"))
print(game_conf)

# To keep things simple, players will write to this path
game_path = spath([b"game", game_pkt.hash])

# We build a query that we'll use as a template for packets relevant for our current game
common_q = lk_query_parse(lk_query(),"domain:=:mineweeper","group:=:[group]","path:=:[0]", pkt=game_pkt, argv=[game_path])

# The [group] and [0] resolved to the bytes from the pkt and argv arguments respectively.
print(lk_query_print(common_q, True))

# Pull signals the exchange process to start gathering the data from the query.
# For queries that aren't pulled, wid's only clash _per thread_.
# We are pulling this query, and we might be playing two games at once.
# By using the game_pkt.hash we ensure its unique enough.
pull_q = lk_query_push(common_q,"","wid", b"game"+game_pkt.hash)
# lk_query_push and *_parse can both achieve the same result.
# The equivalent in lk_query_parse would be:
# pull_q = lk_query_parse(common_q,":wid:game[hash]", pkt=game_pkt)
# Sometimes one is more convenient than the other.

# Signal the exchange process to gather the relevant packets.
# Currently this is silently ignored if no exchange process is running. (might change in the future)
lk_pull(lk, pull_q)

# The bytes we'll be exchanging will all be put into the same type of packet with some fixed arguments.
def new_keypoint(data: bytes | str =bytes(), links=[]):
    return lk_keypoint(
        key=key, domain=b"mineweeper", group=game_pkt.group, path=game_path,
        data=data, links=links
    )
# Pro tip: we could have written
# new_keypoint = functools.partial(lk_keypoint, key=key, domain=b"mineweeper", group=game_pkt.group, path=game_path)


player_count = len(game_conf['players'])
for i, e in enumerate(game_conf['players']):
    print(i, e)

players = list([e[0] for e in game_conf['players']])

game = MineWeeper(    players=players,
    rows=game_conf['rows'],
    cols=game_conf['columns'],
    mine_rate=game_conf['mine_rate'],
    seed=game_pkt.hash)
input("Coordinates are row/vertical, col/horizontal.")

prev_ptr = Link("prev", game_pkt.hash)
"""
The game is a sequence of turns.
The data we're interested in, the cell the current player has chosen, are json encoded [row, col].

Next is the part that requires some creativity.
We have to design a method to encode 'turns'.
There is no right way to do it, and it's an art to find a structure that fits your usecase best.

For MineWeeper we'll keep it simple.
The player whose turn it is should create a packet with his move, and it should also link back to the previous turn. 
(Starting with this dummy prev_ptr for the first round)

We know the order we'll be playing, so we'll look for a packet signed by the current player pointing with the prev_ptr link
"""

def find_and_do_next_move(player_turn_pkt):
    """
    This callback is used for all packets matching our query where the pubkey equals that of the player whose turn it is.
    We'll run it on the database and on new incoming packets.
    Once we find the correct packet pointing to the previous move, we stop looking (return False)
    """
    global prev_ptr, game
    if player_turn_pkt.links[0] == prev_ptr:
        try:
            [row, col] = json.loads(player_turn_pkt.data.decode("utf-8"))
            clear_screen()
            game.reveal(row, col)
            prev_ptr = Link("prev", player_turn_pkt.hash)
            return False #stop looking
        except Exception as e :
            exit("Game was corrupted: " + str(e) )

while game.print_game_state():
    round = game.game_round

    [pid,name] = game.current_player();
    [_name,b64_key] = game_conf['players'][pid]
    # Build a query for packets by the current player.
    # We don't have to pull for this because it's a subset of our game pull.
    # We only have to lk_get_all(for the current db)
    q = lk_query_parse(common_q,"pubkey:=:"+b64_key)

    # By looking for matches that already exists, we ensure a player can quit and 'rejoin' the game by relaunching the process
    lk_get_all(lk,q,find_and_do_next_move)
    # if the find_and_do_next_move function found a match we continue to the next round.
    if round != game.game_round:
        continue

    # If the current players key is our key, we should make a move.
    if b64_key == b64(key.pubkey):
        print("Your turn")
        try:
            [row,col] = re.split('[;|, :]',input("Reveal:"))
            data=json.dumps([int(row),int(col)])
            pkt = new_keypoint(data,links=[prev_ptr])
            print("Cheating",game.is_mine(int(row),int(col)))
            if "y" in input("?"):
                lk_save(lk,pkt)
                prev_ptr = Link("prev",pkt.hash)
            # lk_process updates our database pointer to the latest packet
            # and runs any callbacks setup with lk_watch.
            # In this case there are none.
            # but 'input("?")' might have taken the user a long time, and we want the latest view of the world when we.
            lk_process()
                
        except Exception as e:
            print(e)
    else:
        print("Waiting")
        # A watch always requires a wid. You can overwrite and remove the callback with this wid. 
        q = lk_query_push(q,"","wid","move")

        # Processing packets for a query that we haven't received but are expecting is done with lk_watch.
        # By default lk_watch also iterates over the packets we already received and are in the database.

        # Because we already check the database with lk_get_all packets
        # (and we haven't called either  'lk_procces*' functions which would update its state )
        # We can be slightly more efficient and skip those.
        # The set of packets taken from our database would have an i_db, starting with i_db=0
        # To exclude them, we add the predicate: i_db < 0 
        q = lk_query_push(q,"i_db","<",int(0).to_bytes(4))


        # Whenever the query matches, we run find_and_do_next_move.
        # When a match is found the callback returns False and removes itself.
        lk_watch(lk,q,find_and_do_next_move)
        # lk_process_while is similar to lk_process but will efficiently wait for new packets and process them until a condition is met.
        # Our case is relatively simple.
        # There is only one watch, and only one reason for it to be done.
        # The current player made a move, and we can continue to the next round
        lk_process_while(lk,wid=b"move")
