# Git Infrastructure

`src/infrastructure/git` owns concrete Git command adapters, parser helpers, and
local Git runtime implementations.

Git infrastructure implements application ports such as repo scanning. Domain and
application modules should depend on those ports or domain facts, not on parser
or command-runner details in this folder.
