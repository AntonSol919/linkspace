from typing import Any

DEFAULT_PKT: str
PRIVATE: bytes
PUBLIC: bytes

class Link:
    ptr: Any
    tag: Any
    @classmethod
    def __init__(cls, *args, **kwargs) -> None: ...

class Links:
    idx: Any
    pkt: Any
    @classmethod
    def __init__(cls, *args, **kwargs) -> None: ...
    def __iter__(self) -> Any: ...
    def __next__(self) -> Any: ...

class Linkspace:
    @classmethod
    def __init__(cls, *args, **kwargs) -> None: ...

class Pkt:
    comp0: Any
    comp1: Any
    comp2: Any
    comp3: Any
    comp4: Any
    comp5: Any
    comp6: Any
    comp7: Any
    create: Any
    data: Any
    domain: Any
    group: Any
    hash: Any
    hop: Any
    ipath: Any
    links: Any
    netflags: Any
    path_len: Any
    pkt_type: Any
    point_size: Any
    pubkey: Any
    recv: Any
    signature: Any
    spath: Any
    ubits0: Any
    ubits1: Any
    ubits2: Any
    ubits3: Any
    until: Any
    @classmethod
    def __init__(cls, *args, **kwargs) -> None: ...
    def path_list(self, *args, **kwargs) -> Any: ...
    def __eq__(self, other) -> Any: ...
    def __ge__(self, other) -> Any: ...
    def __getitem__(self, index) -> Any: ...
    def __gt__(self, other) -> Any: ...
    def __hash__(self) -> Any: ...
    def __le__(self, other) -> Any: ...
    def __lt__(self, other) -> Any: ...
    def __ne__(self, other) -> Any: ...

class Query:
    @classmethod
    def __init__(cls, *args, **kwargs) -> None: ...

def b64(*args, **kwargs) -> Any: ...
def lk_datapoint(*args, **kwargs) -> Any: ...
def lk_encode(*args, **kwargs) -> Any: ...
def lk_eval(*args, **kwargs) -> Any: ...
def lk_eval2str(*args, **kwargs) -> Any: ...
def lk_get(*args, **kwargs) -> Any: ...
def lk_get_all(*args, **kwargs) -> Any: ...
def lk_hash_query(*args, **kwargs) -> Any: ...
def lk_info(*args, **kwargs) -> Any: ...
def lk_key(*args, **kwargs) -> Any: ...
def lk_keygen(*args, **kwargs) -> Any: ...
def lk_keyopen(*args, **kwargs) -> Any: ...
def lk_keypoint(*args, **kwargs) -> Any: ...
def lk_keystr(*args, **kwargs) -> Any: ...
def lk_linkpoint(*args, **kwargs) -> Any: ...
def lk_list_watches(*args, **kwargs) -> Any: ...
def lk_open(*args, **kwargs) -> Any: ...
def lk_process(*args, **kwargs) -> Any: ...
def lk_process_while(*args, **kwargs) -> Any: ...
def lk_pull(*args, **kwargs) -> Any: ...
def lk_query(*args, **kwargs) -> Any: ...
def lk_query_clear(*args, **kwargs) -> Any: ...
def lk_query_parse(*args, **kwargs) -> Any: ...
def lk_query_print(*args, **kwargs) -> Any: ...
def lk_query_push(*args, **kwargs) -> Any: ...
def lk_read(*args, **kwargs) -> Any: ...
def lk_save(*args, **kwargs) -> Any: ...
def lk_save_all(*args, **kwargs) -> Any: ...
def lk_status_poll(*args, **kwargs) -> Any: ...
def lk_status_set(*args, **kwargs) -> Any: ...
def lk_watch(*args, **kwargs) -> Any: ...
def lk_write(*args, **kwargs) -> Any: ...
def spath(*args, **kwargs) -> Any: ...
