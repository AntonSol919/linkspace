# v0.2.0
### General
- add ./activate to both the repo and zip for a quick start
- Simplify lk_process_while
- Use env variables: root => LK_DIR, LK_KEY => LK_KEYNAME, LK_PASS
- rename query :id to :qid
- rename comp0-8 into path0-8
- ABE: add `~utf8` function for lossy printing  & use in default_pkt_fmt
### Python
- Improve docs and type hints
- Switch callback semantics, return true to break/finish.
- Improve errors
- impl __hash__, __eq__ etc for more types
### CLI
- lk: change --link option to be a list in tail position (useful with xargs)
- lk: add --pkt-in cli option instead of assuming stdin
