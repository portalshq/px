## Promotion to the Target Branch

Saving a generated candidate on a revision branch does not accept it as canonical. **Promotion requires explicit user acceptance of the version and destination.** A direct request to promote, merge, make canonical, or use this version going forward counts; so does a clear yes to a promotion question. Do not ask again when acceptance is already established. Silence, general thanks, or changing tasks is not acceptance.

After verifying a saved revision, ask briefly whether to promote to the named target branch or keep iterating. At a milestone, one question can cover all outstanding revisions. Continue independent work while a promotion decision is pending. If a downstream task needs an unaccepted revision, resolve which version the user intends before promoting it; do not auto-promote merely because it is a dependency.

## Executing a Promotion

1. Resolve the accepted revision branch and record its commit and representation hashes.
2. Switch to the resolved target branch and check for unrelated dirty content. If applying the revision would overwrite it, report the conflict and preserve the user's work.
3. Apply the accepted representations, properties, references, and provenance as a new target-branch commit. Do not rewrite history or replace unrelated entities.
4. Commit with a message naming the source branch and commit, then push the target branch. Auto-committing tools do not require an extra commit.
5. Resolve the target branch and verify the accepted representation, prompt, reference, and provenance hashes match. Report success only after verification; if push failed, report the local commit and recovery command.

## Reporting

Saved: `PX: <URI> saved on <revision-branch> at <commit>; <key> verified. Promote to <target-branch>, or keep iterating?`

Promoted: `PX: <URI> promoted to <target-branch> at <commit> from <revision-branch> <source-commit>; hashes verified.`

Failed: `PX persistence failed: <URI> generated but not committed` (or `committed locally; push failed`, according to the actual state).
