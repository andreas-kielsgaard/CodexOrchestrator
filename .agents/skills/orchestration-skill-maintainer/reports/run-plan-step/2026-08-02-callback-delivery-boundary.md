# Plan Step callback delivery boundary

## Observation and theory

Several Plan Steps used the host follow-up tool and produced a message in the owner history, while the owner remained idle. One Pause/Restart correction claimed a callback that was not present in the owner history. The worker cannot make session availability or receiver activation true by waiting.

## Revision

The Plan Step makes one callback through a host continuation action when available, records delivery separately from receiver activation, and ends without polling or repeating the callback.

## Evaluation

The revision keeps the worker's action finite and truthful. Receiver activation remains a harness fact rather than a worker responsibility.
