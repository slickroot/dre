# 109: Claim spec numbers atomically

## Problem

Spec numbers are picked by hand as "max + 1". When two agents add specs in
parallel they can pick the same number. Git does not catch it, because
`107-foo.md` and `107-bar.md` are different paths (`028` already exists three
times in the archive, and `105` had to be renumbered to `106`). This is a
technical spec, not a user story: it has no user-facing behaviour.

## Acceptance Criteria

- `scripts/new-spec <slug>` reads the spec body from stdin, writes it to
  `docs/specs/NNN-<slug>.md` and prints that path.
- `NNN` is the highest number found in `docs/specs/` and
  `docs/specs/archive/`, plus one, zero-padded to three digits.
- Parallel invocations never produce the same number.
- The script exits non-zero, writes nothing and explains itself on stderr when
  the current branch is not `main`.
- The script exits non-zero when the slug is missing or is not lowercase
  words joined by hyphens, and when stdin is empty.
- `CONTRIBUTING.md` documents the rule: add specs only on `main`, only via
  `scripts/new-spec`, never by choosing a number by hand.
- `~/.claude/skills/xp-stories/SKILL.md` pipes the story into
  `scripts/new-spec` instead of computing `[spec-number]` itself.

## Technical Design

### Decisions

- Numbers stay. The fix is to make claiming one atomic, not to remove it.
- Claim and write happen in one step under a lock. A script that only printed
  the next number would leave a gap between "number chosen" and "file
  written" in which another agent gets the same number.
- Specs are handled only on `main`, in one checkout, so the scan covers only
  that checkout's `docs/specs/` and `docs/specs/archive/`. No worktree
  enumeration and no git-history scan. Numbers freed by a deleted spec may be
  reused, and that is accepted.
- The script is plain bash. It is workflow tooling, not product code, and it
  needs no build step.

### `scripts/new-spec`

Usage: `scripts/new-spec <slug> < body.md`

1. **Validate arguments.** Require exactly one argument matching
   `^[a-z0-9]+(-[a-z0-9]+)*$`. Read stdin into a variable before taking the
   lock, so a slow producer never holds it, and fail if it is empty.
2. **Guard the branch.** `git rev-parse --abbrev-ref HEAD` must be `main`.
   Otherwise print `specs are only added on main` to stderr and exit 1.
3. **Take the lock.** `mkdir "$(git rev-parse --git-dir)/spec.lock"` is
   atomic. On failure, retry every 50 ms for up to 10 s, then exit 1.
   `trap 'rmdir "$lock"' EXIT` releases it on success, error and Ctrl-C.
   The trap is installed only after `mkdir` succeeds, so a process that
   never got the lock cannot remove someone else's.
4. **Compute the number.** Take the leading three digits of every
   `docs/specs/*.md` and `docs/specs/archive/*.md`, force base 10
   (`10#$n`, so `008` is not read as octal), take the max (0 when there are
   none), add 1 and format with `printf %03d`. Duplicates already in the
   archive (`028`) do not matter.
5. **Write.** Write the body to `docs/specs/NNN-<slug>.md` and print the
   path on stdout. Refuse to overwrite if the file already exists.
6. **Release** via the trap.

The script does not `git add` or commit. Committing stays with the caller,
as today.

### Test (`scripts/new-spec.test.sh`)

Runs against a throwaway git repo created in `mktemp -d` on branch `main`,
with a copy of the script:

- **Numbering:** an empty tree yields `001`; a tree with `009` in `specs/` and
  `012` in `archive/` yields `013`; `008` is not treated as octal.
- **Concurrency:** launch 20 invocations in parallel with distinct slugs and
  assert the 20 printed paths have 20 distinct numbers with no gaps.
- **Branch guard:** on branch `feature`, the script exits non-zero and creates
  no file.
- **Argument validation:** a missing slug, an uppercase slug and empty stdin
  each exit non-zero and create no file.
- **Lock release:** after a failed run the lock directory is gone.

Wired into `make test` next to the Rust tests.

### Docs and skill

- `CONTRIBUTING.md`: new "Adding a spec" section stating the rule and the
  `scripts/new-spec <slug> < body.md` invocation.
- `~/.claude/skills/xp-stories/SKILL.md` (outside the repo, changed as a
  separate step with the diff shown): the line that writes to
  `docs/specs/[spec-number]-[story-slug].md` becomes "pipe the confirmed
  story into `scripts/new-spec <story-slug>` and use the printed path". The
  `xp-tech-design` skill needs no change, because it edits an existing file.

### Out of scope

- Renumbering the existing duplicate `028` files.
- A CI check for duplicate numbers.
