---
name: release
description: Prepare a trevokeyless package release (version sync, changelog, checks) following RELEASING.md. Preparation only — never creates or pushes tags.
argument-hint: "<rust|python|ts> <X.Y.Z>"
---

# Release preparation

Stable X.Y.Z only (the helper rejects prerelease/build metadata). Check the semver
floor first:

- new public API since the last release → at least MINOR
- breaking API change, or a raised toolchain floor (MSRV, Node engines, minimum
  Python) → MAJOR

Steps:

1. `python3 .github/scripts/release_metadata.py --set <rust|python|ts> X.Y.Z`
   (rewrites every version sync point for the package — manifest, lockfiles, derived
   declarations).
2. Review the resulting diff, the lockfile changes especially.
3. In the package's CHANGELOG.md, replace `## [Unreleased]` with
   `## [X.Y.Z] — YYYY-MM-DD` and confirm everything shipping in the release is listed
   (a publish tag fails its gates while the release heading is missing).
4. `python3 .github/scripts/release_metadata.py` — the checker must pass.
5. Run the gates skill for the package.
6. Land the change on `main` through a pull request.

**HARD STOP after the PR.** Do NOT create or push `rust-vX.Y.Z` / `python-vX.Y.Z` /
`ts-vX.Y.Z` tags — a tag push IS the publish trigger. The human tags the merge commit
on main. If several packages release together, tags go one at a time: the publish queue
is one deep, and a third rapid tag silently cancels the pending run (recovery steps in
RELEASING.md; publish-watch.yml files an issue when it happens).
