Output recorded from Claude Code 2.1.282 in stream-json mode with `--replay-user-messages`
and `--permission-prompt-tool stdio`, one scenario per file. Only Claude's output is kept.
Thinking signatures, account details and unused fields are removed, the working directory is
`/work`, and `initialize.jsonl` keeps only the `models` of the `initialize` report.

- `reply`: a plain reply.
- `tool`: an allowed Bash call.
- `approval`: a Bash call behind a permission prompt, allowed once.
- `question`: `AskUserQuestion`, answered with "Blue".
- `interrupt`: an `interrupt` sent after the first assistant message.
- `steer`: a second user message sent while the Bash call ran.
