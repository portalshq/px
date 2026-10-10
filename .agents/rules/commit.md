---
trigger: always_on
---

# Continuous Stacked-Commit Workflow Rules

This workspace operates on a **Continuous Stacked-Commit Workflow**. The goal is to maximize agent development velocity while providing the human reviewer with clean, isolated, and incremental chunks of code to inspect without blocking execution.

## 🚀 Core Rules for the Agent

### 1. Work Linearly on a Single Feature Branch
* Do **not** create multiple branches or split-off draft PRs for individual components.
* Stay on the current feature branch and build sequentially.

### 2. Commit in Atomic "Chunks"
* Group your work into small, cohesive units (e.g., one component, one refactor task, or one module at a time).
* **Never combine unrelated tasks** into a single commit. 
* Every chunk must result in a clean, isolated commit.

### 3. Verify Before Committing
* Before staging any files, verify that the **development server compiles without errors** and all active tests pass.
* Never commit broken code to the timeline.

### 4. Use Conventional Commit Messages
Format every commit message clearly so the reviewer can understand the scope at a glance:
* `feat(component-name): implement specific UI layout`
* `refactor(module-name): optimize state management`
* `fix(component-name): resolve rendering bug`

### 5. Push Immediately
* Run `git push origin <branch-name>` immediately after every local commit. This allows the reviewer to track your progress live via the remote repository platform.

## 🏁 Workflow Completion

When the overall assigned workload or plan is completely finished, you must automatically summarize your work for the reviewer. 

1. **Execute the Summary Script:** Run the workspace utility script located at [scripts/generate-review-summary.sh](scripts/generate-review-summary.sh "scripts/generate-review-summary.sh") to package your commit timeline.
2. **Present the Summary:** Output the results of the script directly in your final response to the user, signaling that the workload is ready for final review.

# Git Commit Attribution

All AI-assisted commits must include attribution for the agent app and model used. This ensures transparency and traceability of AI contributions.

### Required Format

Add the following footer to every commit message when AI assistance was used:

```
Co-Authored-By: <agent-app> (<model>)
```

### Examples

```
feat: add user authentication flow

Co-Authored-By: opencode (nemotron-3-ultra-free)
```

```
fix: resolve race condition in data sync

Co-Authored-By: codex (gpt-4o)
```

```
refactor: extract shared validation logic

Co-Authored-By: devin (devin-1.0)
```

### Agent App Values

Use one of these standard identifiers:
- `opencode` - OpenCode CLI agent
- `codex` - GitHub Copilot / Codex
- `devin` - Devin AI
- `claude` - Claude Code / Claude Desktop
- `cursor` - Cursor IDE
- `windsurf` - Windsurf IDE
- `other` - Any other AI agent (specify)

### Model Values

Use the actual model identifier (e.g., `nemotron-3-ultra-free`, `gpt-4o`, `claude-3.5-sonnet`, `devin-1.0`, etc.)
