# Git Subtree Guidelines

## What is Git Subtree?

Git subtree is a strategy for managing project dependencies as subdirectories within a repository. Unlike submodules, subtrees embed the entire history of the external project directly into your repository, making it easier to track changes and work offline.

### Subtree vs Submodule

| Aspect | Subtree | Submodule |
|--------|---------|-----------|
| Storage | Full history embedded | Pointer to commit |
| Offline work | Fully available | Requires fetch |
| Complexity | Simpler workflow | More complex |
| Repository size | Larger (full history) | Smaller |
| Updates | Merge-based | Checkout-based |
| Cloning | Single clone | Requires `--recursive` |

## When to Use Subtree

**Use subtree when:**
- You need to modify the external code frequently
- You want a simpler workflow for your team
- Offline access to the full history is important
- You're embedding a relatively small repository

**Consider alternatives when:**
- The external repository is very large
- You rarely need to modify the external code
- You need strict version pinning

## Setting Up a Subtree

### Step 1: Add a Remote (Recommended)

```bash
git remote add <remote-name> <repository-url>
```

Example:
```bash
git remote add upstream-lib https://github.com/example/library.git
```

### Step 2: Add the Subtree

**Option A: With full history**
```bash
git subtree add --prefix=<directory-path> <remote-name> <branch>
```

**Option B: Squashed (single commit)**
```bash
git subtree add --prefix=<directory-path> <remote-name> <branch> --squash
```

The `--squash` option compresses the entire subtree history into a single commit, which keeps your repository smaller but loses the detailed history.

### Example: Adding a Subtree

```bash
# Add remote
git remote add cosmic-cliphoard git@github.com:al-ula/cosmic-cliphoard.git

# Add subtree with full history
git subtree add --prefix=cosmic-cliphoard-master cosmic-cliphoard master
```

## Daily Workflow

### Pulling Upstream Changes

When the remote repository has updates you want to incorporate:

```bash
git subtree pull --prefix=<path> <remote-name> <branch>
```

**With squashed history:**
```bash
git subtree pull --prefix=<path> <remote-name> <branch> --squash
```

**Example:**
```bash
git subtree pull --prefix=cosmic-cliphoard-master cosmic-cliphoard master
```

### Pushing Changes Upstream

When you've made changes to the subtree that should be shared:

```bash
# 1. Make your changes and commit normally
git add <subtree-path>
git commit -m "fix: resolve issue in subtree code"

# 2. Push to the subtree's remote
git subtree push --prefix=<path> <remote-name> <branch>
```

**Example:**
```bash
git add cosmic-cliphoard-master/cliphoard/src/app.rs
git commit -m "fix: resolve popup positioning issue"
git subtree push --prefix=cosmic-cliphoard-master cosmic-cliphoard master
```

### Working on Mixed Changes

When you have changes in both the root repository and the subtree:

```bash
# Recommended: Commit separately for clarity
git add <root-files>
git commit -m "docs: update root documentation"

git add <subtree-path>
git commit -m "feat: add new feature to subtree"

# Push subtree changes if needed
git subtree push --prefix=<path> <remote> <branch>
```

## Best Practices

### 1. Use Clear Commit Messages

Prefix commits that affect the subtree to make tracking easier:

```
[subtree] feat: add new feature
[subtree] fix: resolve bug
[subtree] chore: update dependencies
```

### 2. Pull Before Push

Always pull upstream changes before pushing to avoid conflicts:

```bash
git subtree pull --prefix=<path> <remote> <branch>
# Resolve any conflicts if they occur
git subtree push --prefix=<path> <remote> <branch>
```

### 3. Keep Subtree Changes Isolated

Avoid mixing subtree changes with root repository changes in the same commit. This makes it easier to:
- Track what changes go upstream
- Revert changes if needed
- Review commit history

### 4. Document Your Subtrees

Create a section in your README or documentation:

```markdown
## Subtrees

| Path | Remote | Branch | Purpose |
|------|--------|--------|---------|
| `lib/external` | `https://github.com/org/repo` | `main` | Shared utilities |
```

### 5. Use Git Aliases

Add convenient aliases to your `.gitconfig`:

```ini
[alias]
    # Generic subtree aliases
    sub-add = "!f() { git subtree add --prefix=$1 $2 ${3:-main}; }; f"
    sub-pull = "!f() { git subtree pull --prefix=$1 $2 ${3:-main}; }; f"
    sub-push = "!f() { git subtree push --prefix=$1 $2 ${3:-main}; }; f"
```

Usage:
```bash
git sub-pull lib/external upstream-lib main
```

## Troubleshooting

### "refusing to merge unrelated histories"

This occurs when Git cannot find a common ancestor between your repository and the subtree.

**Solution:**
```bash
git subtree pull --prefix=<path> <remote> <branch> --allow-unrelated-histories
```

### Merge Conflicts During Pull

When upstream changes conflict with your local modifications:

```bash
# 1. Check which files are conflicted
git status

# 2. Open each conflicted file and resolve the markers
# <<<<<<< HEAD
# your changes
# =======
# upstream changes
# >>>>>>> <commit-hash>

# 3. Stage resolved files
git add <resolved-files>

# 4. Complete the merge
git commit
```

### Push Rejected (Non-Fast-Forward)

This happens when the remote has commits that aren't in your local subtree.

**Solution:**
```bash
# Pull first to get remote changes
git subtree pull --prefix=<path> <remote> <branch>

# Resolve any conflicts, then push
git subtree push --prefix=<path> <remote> <branch>
```

### Subtree Command is Slow

For large repositories, subtree operations can be slow. Consider:

1. Using `--squash` to reduce history
2. Shallow clones if full history isn't needed
3. Splitting large subtrees into smaller ones

### Accidentally Committed to Wrong Location

If you committed changes meant for the subtree to the root:

```bash
# The commit is still in the subtree path, so you can still push it
git subtree push --prefix=<path> <remote> <branch>
```

If you committed subtree changes outside the subtree path, you'll need to move the files and amend the commit.

## Advanced Operations

### Splitting a Subtree

Extract a directory into a separate subtree:

```bash
git subtree split --prefix=<path> -b <new-branch>
```

This creates a new branch containing only the history of that directory.

### Merging a Subtree into a Different Branch

```bash
git subtree merge --prefix=<path> <remote>/<branch>
```

### Viewing Subtree History

```bash
# View commits affecting the subtree path
git log --oneline -- <path>

# View with patch details
git log -p -- <path>
```

## Removing a Subtree

To remove a subtree from your repository:

```bash
# Remove the directory
git rm -rf <subtree-path>

# Commit the removal
git commit -m "chore: remove subtree <name>"

# Optionally remove the remote
git remote remove <remote-name>
```

Note: The subtree's history will remain in your repository's git history unless you rewrite history (not recommended for shared repositories).

## Quick Reference Card

```
┌─────────────────────────────────────────────────────────────────┐
│                    GIT SUBTREE QUICK REFERENCE                   │
├─────────────────────────────────────────────────────────────────┤
│ SETUP                                                           │
│   git remote add <name> <url>                                   │
│   git subtree add --prefix=<path> <remote> <branch> [--squash]  │
├─────────────────────────────────────────────────────────────────┤
│ DAILY OPERATIONS                                                │
│   git subtree pull --prefix=<path> <remote> <branch> [--squash] │
│   git subtree push --prefix=<path> <remote> <branch>            │
├─────────────────────────────────────────────────────────────────┤
│ TROUBLESHOOTING                                                 │
│   Add --allow-unrelated-histories for unrelated history errors  │
│   Always pull before push to avoid non-fast-forward errors      │
├─────────────────────────────────────────────────────────────────┤
│ BEST PRACTICES                                                  │
│   • Pull before push                                            │
│   • Keep subtree commits isolated                               │
│   • Use clear commit message prefixes                           │
│   • Document your subtrees                                      │
└─────────────────────────────────────────────────────────────────┘
```

## This Project

This repository uses a subtree for `cosmic-cliphoard-master/`:

```bash
# Pull from upstream
git subtree pull --prefix=cosmic-cliphoard-master cosmic-cliphoard master

# Push to upstream
git subtree push --prefix=cosmic-cliphoard-master cosmic-cliphoard master
```

## See Also

- [Git Subtree Documentation](https://git-scm.com/book/en/v2/Git-Tools-Advanced-Merging#_subtree_merge)
- [plans/monorepo-migration.md](./plans/monorepo-migration.md) - Detailed migration plan for this project
