import linkspace
import itertools
import re
from dataclasses import dataclass

class NoSuchCell(Exception): pass
class AlreadyRevealed(Exception): pass

def clear_screen():
    print("\n"*100)

@dataclass
class Cell:
    mine: bool = False
    revealed: bool = False 
    neighbour_mines:int = 0
    def __str__(self):
        return (not self.revealed and ".") or (self.mine and "X") or str(self.neighbour_mines)


class MineWeeper():
    players: list[str]
    rows:int
    cols:int
    mine_rate:float
    seed: bytes
    grid:list[list[Cell]] =[]
    loser = None
    game_round = 0

    def __init__(self, players: list[str], rows: int, cols: int, mine_rate: float, seed: bytes) -> object:
        self.players = players
        self.rows = rows;
        self.cols = cols
        self.mine_rate = mine_rate
        self.seed = seed
        self.grid =  [ [Cell(self.is_mine(i,j)) for j in range(self.cols)]  for i in range(self.rows)]
        for r,row in enumerate(self.grid):
            for c,cell in enumerate(row):
                for (dr,dc) in [(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)]:
                    neighbour = self.get_cell(r+dr,c+dc)
                    if neighbour:
                        cell.neighbour_mines += neighbour.mine

    def current_player(self) -> tuple[int,str]:
        player_id = self.game_round % len(self.players)
        return (player_id,self.players[player_id])

    def is_mine(self,row:int,col:int) -> bool:
        rand_bytes = linkspace.blake3_hash(self.seed + row.to_bytes(length=4) + col.to_bytes(length=4))
        random = linkspace.bytes2uniform(rand_bytes)
        return random < self.mine_rate

    def get_cell(self,row,col)  -> Cell | None:
        if row >= 0 and row < self.rows and col >= 0 and col < self.cols:
            return self.grid[row][col]
        return None

    def print_grid(self):
        coord_fmt = "[{:>2}]"
        print(*["[##]"] + [coord_fmt.format(c) for c in range(self.cols)])
        for r,row in enumerate(self.grid):
            print(*[coord_fmt.format(r)] + [ " {:>2} ".format(str(cell)) for cell in row])

    def count_revealable(self) -> int:
        """number of cells left to reveal"""
        return sum([
            not cell.revealed and not cell.mine
            for row in self.grid for cell in row
        ])


    def print_game_state(self) -> bool:
        """Print board and current state. returns False if game is finished"""
        print(self.print_grid())
        print("Round ",self.game_round)
        if self.loser is not None:
            print("Game Finished!")
            print("The loser is {} ({})".format(self.players[self.loser],self.loser))
            return False
        if self.count_revealable() == 0:
           print("Everybody survived!")
           print("for now ...")
           return False
        print(self.count_revealable(),"options to not lose")
        (pid,name) = self.current_player()
        print("Player {} ({})".format(name,pid))
        return True

    def get_revealable_cell(self,row,col) -> Cell:
        """Get unrevealed cell at row,col. Raises exceptions if not possible"""
        cell = self.get_cell(row,col)
        if cell is None:
            raise NoSuchCell
        elif cell.revealed:
            raise AlreadyRevealed
        return cell


    def reveal(self,row,col):
        cell = self.get_revealable_cell(row,col)
        cell.revealed = True
        (pid,_) = self.current_player()
        if cell.mine:
            print(f"({row},{col}): Boom!!!")
            self.loser = pid
        else:
            print(f"({row},{col}): Phew.")
            self.game_round += 1
