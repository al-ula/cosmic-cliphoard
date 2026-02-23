This will be a workspace for creating a wl-clipboard based clipboard manager for cosmic desktop.
This directory will also used to work on all the worktrees including the master worktree.
We will have a directory to work on the master branch of the repo which is a cargo workspace.
The cargo workspace will include these crates:
 - The main decoder crate, similar to cliphist, which can be compiled as library crate, cdylib, and cli.
 - A shared crates that will be used by two different runtime. An applet and a tray app (for non cosmic).
 - The layer shell ui (shared).
 - wl-clipboard managed by applet as child process.
 - in case user prefer system tray user need to manually enable a unit file.
