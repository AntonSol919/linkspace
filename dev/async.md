Async is currently messy and broken but not an immediate priority.
Its also not exposed in liblinkspace.

# goal
A user should be able to spawn a future on the same executor as the linkspace runtime is waiting for
new packets.

In its current state this is done with the .spawner field in the linkspace.
Currently the lk_process_while skips this runtime entirely.
