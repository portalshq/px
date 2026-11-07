# Harness skill adapters

PX uses skill directories for harness-neutral instructions. The agents adapter targets `~/.agents/skills`; Claude Code targets `~/.claude/skills`, and Cursor targets `~/.cursor/skills`. Cursor documents both its own directory and the shared agents directory in its [skills documentation](https://cursor.com/help/customization/skills). Claude Code documents personal skills in its [skills guide](https://code.claude.com/docs/en/skills).

The proposed Devin path is `~/.config/devin/skills`. Its global discovery behavior has not yet been verified against current Devin documentation; do not assume that writing to this directory activates a skill. Devin documents repository-local `.agents/skills` in its [repository setup guidance](https://docs.devin.ai/onboard-devin/repo-setup).

Skill installation must not overwrite existing directories. PX marks its own installed copies so removal cannot silently delete a user-owned skill. Hook declarations are scaffolding only: PX prints manual instructions and does not edit harness settings.
