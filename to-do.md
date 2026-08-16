# Worktree To Do

## Scope

Scratch is a lifecycle and creation mode, not a separate kind of worktree.
The same discovery, status, path, and entry utilities should work for the base worktree, named branch worktrees, and detached scratch worktrees.

Named worktrees must retain their branch-oriented behavior.
Scratch-specific behavior must never silently reset or delete a named worktree.

## General Worktree Utilities

- [x] Make `bp worktree list` show every worktree with a stable name or path, branch or detached state, clean or dirty state, current or in-use state, and relevant tmux, process, remote, and pull-request information.
- [x] Add a generic `bp worktree enter <NAME|PATH>` operation that opens an existing worktree without resetting it or changing its lifecycle state.
- [x] Make `bp worktree path <NAME|PATH>` work for base, named, and scratch worktrees and keep its output clean for shell integrations.
- [ ] Add machine-readable output for worktree discovery and status.
- [x] Add generic process and active-session detection that can be reused by attach, return, remove, and prune operations.
- [x] Define clear lifecycle states such as current, available, in-use, dirty, detached, leased, and unverified.

## Scratch Worktree Lifecycle

The existing no-argument `bp worktree new` and `bp worktree n` commands create detached scratch worktrees.
The following tasks make those worktrees reusable and safely manageable.

### Return and Reuse

- [x] Detect available scratch worktrees before creating a new one.
- [x] Treat a scratch worktree as reusable only when it is clean, detached, valid, and not currently in use.
- [x] Add a return operation that can infer the current worktree when invoked from inside it.
- [x] Refuse to discard tracked or untracked changes without confirmation.
- [x] On return, detach `HEAD`, remove tracked and ordinary untracked changes, and preserve ignored build caches and dependencies where possible.
- [x] Mark the returned scratch worktree available for the next no-argument `new` or `n` operation.
- [x] Run normal worktree link synchronization and scratch setup after reuse.
- [x] Ensure two concurrent acquisitions cannot select the same scratch worktree.

### In-Use Detection

- [x] Detect processes whose working directory is inside a worktree.
- [x] Detect active tmux sessions and other relevant local ownership signals.
- [x] Keep process use, short-lived lifecycle reservations, and durable leases as separate facts.
- [x] Refuse automatic reuse, return, or destruction while a worktree is in use unless an explicit override is provided.
- [x] Show the process or session reason when a worktree is skipped.

### Removal and Destruction

- [x] Make removal accept exact names and paths for nested scratch worktrees.
- [x] Make scratch removal avoid branch deletion because scratch worktrees are detached and branchless.
- [x] Keep returning a worktree for reuse separate from permanently destroying it.
- [x] Add a dry-run preview for destructive removal.
- [x] Protect dirty, in-use, leased, and unverified worktrees by default.
- [x] Require separate explicit options for destructive risk classes such as including dirty or in-use worktrees.

### Pruning

- [x] Extend pruning to inspect scratch worktrees as well as named worktrees.
- [x] Remove only clean, available, detached scratch worktrees by default.
- [x] Keep pruning a dry run unless explicitly confirmed.
- [x] Report skipped worktrees with actionable reasons.
- [x] Add optional age-based cleanup for scratch worktrees.
- [x] Report reclaimed disk space.

### Persistent State and Recovery

- [x] Store scratch ownership, availability, and last-used metadata in a small locked state file.
- [x] Write state atomically so an interrupted lifecycle operation cannot make a worktree appear safely reusable.
- [x] Quarantine scratch worktrees when state is missing, corrupt, or cannot prove that reuse is safe.
- [x] Add a recovery path that lets a user inspect and explicitly release or destroy quarantined worktrees.

## Suggested Implementation Order

1. Generic worktree discovery and state rendering.
2. In-use detection and concurrency reservation.
3. Return and clean a scratch worktree for reuse.
4. Reuse an available scratch worktree from `new` and `n`.
5. Safe scratch removal and destruction.
6. Scratch-aware pruning and disk-space reporting.
7. Persistent state recovery and optional durable leases.
