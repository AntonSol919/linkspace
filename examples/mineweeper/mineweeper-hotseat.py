from mineweeper import MineWeeper, clear_screen, NoSuchCell,AlreadyRevealed
import re,random

player_count = int(input("Number of players?:"))
players = [ input("Enter name>:") for _ in range(player_count)]
rows = 20 # int(input("rows"))
cols = 20 # int(input("cols"))
mine_rate = 0.3 # int(input("mine_rate"))

# A game started with the same (rows,cols,seed) will have the mines at the same location
seed = random.randbytes(4)

game = MineWeeper(players,rows,cols,mine_rate,seed=seed)

input("Coordinates are row/vertical, col/horizontal. [Enter to start]")
while game.print_game_state():
    try:
        [row,col] = re.split('[;|, :]',input("Reveal:"))
        clear_screen()
        game.reveal(int(row),int(col))
    except Exception as e:
        print(e)
