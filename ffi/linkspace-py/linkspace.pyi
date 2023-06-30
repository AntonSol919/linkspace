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

"""An linkspace packet: netheader, hash, and point - all fields are in (big endian) bytes"""
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
    pubkey: bytes
    recv: bytes
    signature: bytes
    ubits0: bytes
    ubits1: bytes
    ubits2: bytes

    ubits3: bytes
    """ [u64:0] - microseconds since epoch"""
    stamp: bytes

    """ [u16:0] - the size of the packet as it would be using lk_write"""
    size: bytes
    
    def path_list(self) -> list[bytes]: ...
    def __eq__(self, other) -> bool: ...
    def __ge__(self, other) -> bool: ...
    def __getitem__(self, index) -> Any: ...
    def __gt__(self, other) -> bool: ...
    def __hash__(self) -> int: ...
    def __le__(self, other) -> bool: ...
    def __lt__(self, other) -> bool: ...
    def __ne__(self, other) -> bool: ...


def lk_datapoint(data:bytes) -> Pkt: ...
def lk_linkpoint(group:bytes|None=None,domain:bytes|str|None=None,path:bytes|str|list[bytes|str]|None=None,
                 links:list[Link] | None=None,data:bytes |str| None=None,
                 create:bytes | None =None) -> Pkt: ...
def lk_keypoint(key: SigningKey,
                group:bytes|None=None,domain:bytes|str|None=None,path:bytes|str|list[bytes|str]|None=None,
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

def lk_get_hash(lk:Linkspace,hash:str|bytes) -> Pkt | None:
    """Read a single packet hash from the database"""
    ...


def lk_hash_query(hash:str|bytes) -> Query:
    """
    Shorthand for building a query that matches a single hash
    Use lk_get_hash if you do not care to watch the database
    """
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
    Registers the query under its 'qid' ( .e.g. set by lk_query_parse(q,":qid:myqid) )
    Before returning, calls on_match for every packet in the database.
    The absolute return value is the number of times the callback was called.

    The watch is finished when
    - the cb returns 'break' (In other languages we map this to the boolean 'true')
    - the predicate set will never match again (e.g. the 'i_*' or 'recv' predicate shall never match again )
    - [[lk_stop]] is called with the matching id

    Args:
        lk:
        query: A query used in lk_watch requires the 'qid'  option be set ( lk_query_parse(q,":qid:myqid") )
        on_match:
        on_close:
        on_err:
    Returns: int
        Positive if callback returned 'True', negative or 0 if not.
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

def lk_status_poll(lk:Linkspace,qid:bytes,objtype:bytes,
                   timeout:bytes, 
                   instace : bytes | None = None ,
                   callback:Callable[[Pkt],Any] | None = None,
                   group:bytes|None = None,domain:bytes|None=None
                   ) -> bool:
    """
    status_set and status_poll are a convention to communicate between two processes over the [#:0] group about a (group,domain,obj_type, ?instace).
    I.e. allows multiple processes to loosely communicate by agreeing on a objtype name and what the status packets should contain. 

    The status convention is only meant for communication between processes using the same linkspace instance.
    lk_status_poll accepts any reply made between [now-timeout .. now+timeout].

    This function is an application of lk_watch. An immediate check is made of the current database.
    For further processing callback is registered under qid, and is only executed during a lk_process* step. 

    For example, an exchange process must lk_status_set for (group,b"exchange","process", exchangename).
    An application can lk_status_poll a (group,b"exchange",b"process") to determine if a processes is running.

    If no instance is set, then all lk_status_set with the same (group,domain,obj_type) will reply.

    A minimal example looks like: 
    # 
    immediate_reply = lk_status_poll(lk,qid=b"status",callback=lambda _ : True,
               timeout=lk_eval("[s:+2s]"),
               domain=b"exchange",
               group=group,
               objtype=b"process") 
    if not immediate_reply and lk_process_while(lk,qid=b"status") == 0:
        print("No exchange process active")

    Args:
        lk:
        qid: name to lk_watch. Can be lk_stop or watched with lk_process_while
        timeout: microseconds window to check for reply. e.g. lk_eval("[s:+2s]").
        group: GroupID for the objtype
        domain: domain for the objtype
        objtype: a agreed upon name for the status.
        instance: a specific instance for the objtype
        callback: Receives the status packets made with lk_status_set. Return True to stop early.
    
    Returns: true if any lk_status_set has replied. 
    """
    ...


def lk_status_set(lk:Linkspace,qid:bytes,
                  objtype:bytes,
                   get_current_status:Callable[[],bytes] | None = None,
                   instance : bytes | None = None ,
                   group:bytes|None = None,domain:bytes|None =None,
                   ) -> bool:
    """
    status_set and status_poll are a convention to communicate between two processes over the [#:0] group about a (group,domain,obj_type, ?instace).
    I.e. allows multiple processes to loosely communicate by agreeing on a objtype name and what the status packets should contain. 

    The status convention is only meant for communication between processes using the same linkspace instance.
    lk_status_poll accepts any reply made between [now-timeout .. now+timeout].

    This function is an application of lk_watch. An immediate check is made of the current database.
    For further processing get_current_status is registered under qid, and is only executed during a lk_process* step. 

    For example, an exchange process must lk_status_set for (group,b"exchange","process", exchangename).
    An application can lk_status_poll a (group,b"exchange",b"process") to determine if a processes is running.
    """
    ...



def lk_write(pkt:Pkt) -> bytes:
    """Get the byte representation of the packet."""
    ...
def lk_read(bytes:bytes,validate:bool=True,allow_private:bool=False) -> tuple[Pkt,bytes]:
    """
    Read bytes as a packet.
    Args:
        bytes:
        validate:
        allow_private: Accept [#:0] group packets.
    """
    ...

def b64(bytes:bytes) -> str: ...
def spath(components:list[bytes]) -> Any:
    """
    Encode a list of components in the SPath byte format (the same as Pkt.path).
    """
    ...

def blake3_hash(bytes:bytes) -> bytes: ...

def bytes2uniform(bytes:bytes) -> float:
    """
    Read bytes as a [0,1) float by reading the first 52 bits.
    Primary use is to produces the same 'random',
    regardless of language, and without an additional RNG dependencies,

    Args:
        bytes: (random) 32 bytes e.g. Pkt.hash or blake3_hash()
    Returns:
        float: uniform value ranging from [0,1)
    """
    ...



def set_group(group:bytes):
    """manualy set the default `group()`"""
def group() -> bytes:
    """get the default group bytes - set_group || $LK_GROUP || [#:pub]

    This is a thread local value. Is only determined once and can't be changed afterwards.
    Is used as a default when no group argument is given for various functions.
    """
    ...
def set_domain(domain:bytes):
    """manualy set the default `domain()`"""
def domain() -> bytes:
    """get the default domain bytes - set_domain || $LK_DOMAIN || bytes(16) 

    This is a thread local value. Is only determined once and can't be changed afterwards.
    Is used as a default when no group argument is given for various functions.
    """
    ...
