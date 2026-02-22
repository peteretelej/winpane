# P3: C ABI & FFI - Implementation Prompt

## Setup

1. Read `_docs/initial-implementation/p3-ffi/learnings.md` first for any corrections from previous phases
2. Read `_docs/initial-implementation/p3-ffi/plan.md` to find the next incomplete phase
3. Read the phase's `spec.md` and `plan.md` in the corresponding phase folder
4. Use Todo to track the checklist items for the phase

## Implementation

5. Implement following the phase checklist
6. Update `_docs/initial-implementation/p3-ffi/learnings.md` if you discover corrections or gotchas
7. Run pre-push checks:
   ```bash
   cargo fmt --all -- --check
   cargo fmt --all  # fix if needed
   ```
8. Write a proposed commit message to the phase's `pr.md` file:
   - **Title**: max 50 chars, lowercase, no conventional-commit prefixes (no feat:, fix:, docs:, etc.), no phase references
   - **Body**: one or two short plain sentences on what changed for the user/system. Optional bullets for secondary details only.
   - **Avoid**: file lists, test counts, function names, phase numbers, verbose repetition of the title, "Phase X completed"
9. Mark the phase complete in the checklist and in the root `plan.md`
10. Ask user for review: present `pr.md` content and summary of validation steps

## Reference

- Proposal: `_docs/initial-implementation/p3-ffi/proposal.md`
- Full implementation plan: `_docs/initial-implementation/p3-ffi/initial-plan.md`
- P2 context: `_docs/initial-implementation/phases-progress.md`
