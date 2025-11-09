## Worklog
This is a human-written worklog about the experience of building an application.

AI agents should not modify this document.

### 2025-11-09
- after yesterday's false starts, have started by asking codex cli write a specification for the application into README.md that will later be used to serve as a basis for the code being written (as opposed to providing the agent with only a prompt)
- first attempt failed where the application couldn't complete `cargo test` or `cargo check` and seemed to be failing at the input level, upon reviewing the codex logs the environment was not able to make network requests to download dependences
- after enabling network access for the environment, it was able to download dependencies and see and resolve the issues that i was encountering. network access is very important!
- also found out the hard way that codex cannot update prs that had contributions from me directly, which is annoying lock in
- needed to prompt codex to write docs for its code, and unit tests, as well

### 2025-11-08
- started by creating an AGENTS.md using /init and warp also created a WARP.md
- several false starts here -- tried to use codex cli, and codex web to scaffold an application that uses a basic connection string provided via a .env file, despite several attempts it was never able to successfully complete a request
- the first few times the application wouldn't even compile or pass the tests that were written