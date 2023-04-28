from typing import Any,Callable

DEFAULT_PKT: str
PRIVATE: bytes
PUBLIC: bytes

class Link:
    tag: bytes
    ptr: bytes
    @classmethod
    def __init__(cls,tag: bytes|str , ptr:bytes) -> None:
        """
        A (16 bytes, 32 bytes) tuple for referencing packets

        Args:
            tag: up to 16 bytes
            ptr: 32 bytes hash from a Pkt
        """
        ...
    def __hash__(self) -> int: ...

class Links:
    @classmethod
    def __init__(cls, *args, **kwargs) -> None: ...
    def __iter__(self) -> Any: ...
    def __next__(self) -> Link | None : ...

class Linkspace: ...
class SigningKey:
    pubkey: bytes
class Query: ...

class LkInfo:
    dir : str

class Pkt:
    path: bytes
    """The path bytes in spath encoding format. Use path0 up to path7 or path_list() to get each components."""
    path0: bytes
    path1: bytes
    path2: bytes
    path3: bytes
    path4: bytes
    path5: bytes
    path6: bytes
    path7: bytes
    create: bytes
    data: bytes
    domain: bytes
    group: bytes
    hash: bytes
    hop: bytes
    links: bytes
    netflags: bytes
    path_len: bytes
    pkt_type: bytes
    point_size: bytes
    pubkey: bytes
    recv: bytes
    signature: bytes
    ubits0: bytes
    ubits1: bytes
    ubits2: bytes
    ubits3: bytes
    stamp: bytes

    def path_list(self) -> list[bytes]: ...
    def __eq__(self, other) -> bool: ...
    def __ge__(self, other) -> bool: ...
    def __getitem__(self, index) -> Any: ...
    def __gt__(self, other) -> bool: ...
    def __hash__(self) -> int: ...
    def __le__(self, other) -> bool: ...
    def __lt__(self, other) -> bool: ...
    def __ne__(self, other) -> bool: ...


def b64(bytes:bytes) -> str: ...
def lk_datapoint(data:bytes) -> Pkt: ...
def lk_linkpoint(group:bytes|None=None,domain:bytes|str|None=None,path:bytes|str|None=None,
                 links:list[Link] | None=None,data:bytes |str| None=None,
                 create:bytes | None =None) -> Pkt: ...
def linkmail_keypoint(key: SigningKey,
                group:bytes|None=None,domain:bytes|str|None=None,path:bytes|str|None=None,
                links:list[Link] | None=None,data:bytes|str | None=None,
                create:bytes | None =None) -> Pkt: ...

def lk_eval(abe:str,pkt:Pkt|None=None,argv:list[bytes|str] | None = None ) -> bytes:
    """
    Evaluate an ascii-byte-expression. An ascii representation of arbitrary bytes.
    See the guide and rust docs for examples
    Args:
        abe:
        pkt:
        argv:
    """
    ...

def lk_eval2str(abe:str,pkt:Pkt|None=None,argv:list[bytes|str] | None = None ) -> str:
    """ lk_eval that attempts to utf-8 decode the result bytes into a string"""
    ...

def lk_encode(
        bytes:bytes,
        opts:str|None) -> str:
    """
    Inverse of lk_eval. Encodes bytes into an ascii-byte-expression.
    Args:
        bytes:
        opts: ABE functions to use for encoding.
            "u32/b" tries to encode bytes as [u32:...], then [b:...] (always succeeds)
            If no argument is given or no function can encode bytes successfully then fallback to abtext
    Returns:
        Input bytes encoded in abe format.
    """
    ...


def lk_get(lk:Linkspace,query:Query) -> Pkt | None:
    """Get the first result from the database."""
    ...
def lk_get_all(lk:Linkspace, query:Query,cb:Callable[[Pkt],bool|None]) -> int:
    """
    Run callback for every packet matching the query in the database.
    Stops early if the callback returns True.
    
    Args:
        lk:
        query:
        cb : callable
            - ``pkt``: Pkt
            Returns: bool,optional
    Returns:
        If cb returned True, returns number of packets matching the query. Otherwise -1 * number of matches.
    
    """
    ...
def lk_hash_query(hash:str|bytes) -> Query:
    """Shorthand for building a query that matches a single hash"""
    ...

def lk_info(lk:Linkspace) -> LkInfo:
    """Misc info about the linkspace instance"""
    ...

def lk_key(lk:Linkspace,password:bytes|None=None,name:str|None=None,create:bool=False) -> SigningKey:
    """
    Combine lk_keygen , lk_enckey, and generate or open a SigningKey named through LNS

    Args:
        lk: Linkspace
        password: $LK_PASS | ""
        name: $LK_KEYNAME | "me:local"
        create: Create if it does not exist.
    """
    ...

def lk_keygen() -> SigningKey: ...
def lk_keyopen(enckey:str,password:bytes) -> SigningKey: ...
def lk_enckey(key:SigningKey, password:bytes) -> str: ...
def lk_list_watches(*args, **kwargs) -> Any: ...
def lk_open(dir:str|None = None,create:bool=False) -> Linkspace:
    """
    Open a linkspace instance.

    Args:
        dir: $LK_DIR | $HOME/linkspace
    """
    ...
def lk_process(lk:Linkspace):
    """
    Update the thread view of the database to include new packets saved from other applications and processes.
    This triggers callbacks registered with lk_watch
    Args:
        lk:
    """
    ...
def lk_process_while(lk:Linkspace, qid:bytes|None=None,timeout:bytes|None=None) -> int:
    """
    Continuously await new packets and lk_process until:
    - the timeout has expired:
    - a query with qid and registered with lk_watch was hit at least once.

    timeout
    Args:
        lk: Linkspace instance
        qid: query id
        timeout: u64 microseconds.
            E.g. lk_eval("[s:+1m3s]")
            or int(1000 * 1000 * 63).to_bytes(8)
    Returns:
        0 if a timeout has expired.
        -1 if the qid is hit and is still actively waiting for more.
        1 if the qid is hit and is no longer registered. .
    """
    ...

def lk_query(template:Query | None = None) -> Query: ...
def lk_query_parse(q:Query, *statement : str,
                   pkt:Pkt|None=None, argv:list[bytes|str]|None=None):
    """
    Add one or more statements in ABE format. Use lk_query_push if the encoding step is superfluous.
    Each is evaluated with pkt and argv as context

    See the guide or rust docs for a full list of predicates and options
    """
    ...
def lk_query_push(q:Query, field:str,op:str,val:bytes):
    """
    Add a single predicate to a query, with the val in bytes.
    See the guide or rust docs for a full list of predicates and options.
    """
    ...
def lk_query_print(q:Query, expr: bool = False) -> str:
    """
    Print the query as a list of statements. Can be used in lk_query_parse.
    Args:
        q:
        expr: If true uses expressions like '[b:...]' where possible.
    """
    ...


def lk_save(lk:Linkspace, pkt:Pkt) -> bool:
    """
    Save a packet to the database. Don't forget to lk_process*. 
    Args:
        lk:
        pkt:
    Returns:
        True if packet is new, False if already exists.
    """
    ...

def lk_save_all(lk:Linkspace, pkts:list[Pkt]) -> int:
    """
    lk_save a list of packets.
    Returns:
        the number of new packets saved."""
    ...

def lk_watch(lk:Linkspace,query:Query,
            on_match:Callable[[Pkt],bool|None] ,
            on_close:Callable[[Pkt],Any] | None = None,
            on_err:Callable[[Pkt],Any] | None = None,
             ) -> int:
    """
    Before returning, calls on_match for every packet in the database.
    Then registers the query under its 'qid' option.
    During a lk_process or lk_process_while call, for every new packet matching the query, on_match will be called.

    The watch is deregistered when:
    - on_match returns False
    - lk_stop is called with a matching qid
    - the query is done (The recv predicate is out of bound, the i_* predicate has reached its limit)

    Args:
        lk:
        query: A query used in lk_watch requires the 'qid'  option be set ( lk_query_parse(q,":qid:myqid") )
        on_match:
        on_close:
        on_err:
    """
    ...
def lk_stop(lk:Linkspace, qid:bytes,range:bool=False):
    """Drop the query registered with lk_watch. If range is set, drop all with the qid prefix."""
    ...

def lk_pull(lk: Linkspace, query: Query):
    """
    A convention to signal an exchange process by saving the query to:
         [f:exchange]:[#:0]:/pull/[qgroup]/[qdomain]/[wid]

    Args:
        lk: 
        query: Must have a qid set
    """
    ...
def lk_status_poll(*args, **kwargs) -> Any: ...
def lk_status_set(*args, **kwargs) -> Any: ...



def lk_write(pkt:Pkt) -> bytes:
    """Get the byte representation of the packet."""
    ...
def lk_read(bytes:bytes,validate:bool=True,allow_private:bool=False) -> tuple[Pkt,bytes]:
    """
    Read bytes as a packet. In python this will copy the bytes.
    Args:
        bytes:
        validate:
        allow_private: Accept [#:0] group packets.
    """
    ...

def path(components:list[bytes]) -> Any:
    """
    Encode a list of components in the SPath byte format (the same as Pkt.path).
    """
    ...
