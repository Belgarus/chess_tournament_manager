---
name: emoji-markers
description: Use when formatting conversational assistant output to tag a paragraph, or a parse-back node, with a single status marker (emoji plus uppercase bracketed word), and as the marker vocabulary behind parse-back. Defines a fixed, closed set covering planning, findings, evidence, warnings, decisions, goals, checkpoints, steering, inspection, experiments, approvals, scheduling, pinning, and cleanup. Triggers on pin, park, defer, resume, revisit, remind, clean up, inspect, trial, or batch run.
---

# Emoji Markers

A fixed marker vocabulary for conversational output. Every unit — a paragraph, or a unit of work — is tagged by a marker; a marker may open a unit or close it.

## Canonical form

Start a unit with `EMOJI [WORD]` — the emoji first, then the bracketed uppercase word marker, then the text:

```
🧾 [REPORT] The build finished with two warnings.
```

- Emoji comes first; the bracketed word marker comes second. This order is fixed.
- Show both parts by default.
- Only when the surface cannot render emoji (plain terminals, logs, diffs), fall back to the bare word marker: `[REPORT]`.

## Marker set (closed)

Use only these markers. Never invent, abbreviate, translate, or reorder them. These are the general meanings; the meaning a marker takes as a start-marker, along with the sibling order, is defined by [parse-back](../parse-back/).

| Emoji | Word marker | Meaning |
|---|---|---|
| 📋 | `[FINALIZE]` | Finalized / settled |
| 📎 | `[EVIDENCE]` | Evidence for a claim |
| ✅ | `[OK]` | Acknowledged / verified |
| 🧾 | `[REPORT]` | Report / summary |
| ⚠️ | `[WARN]` | Uncertainty / correction |
| ❓ | `[DECIDE]` | Decision needed |
| 🏁 | `[FINISH]` | Main goal / end state |
| 🚧 | `[CHECK]` | Checkpoint / deliberate pause |
| 🧲 | `[STEER]` | Steer toward a (sub)goal |
| 🧠 | `[PLAN]` | Planning / outlook |
| 🔨 | `[EXEC]` | Executed step |
| 🔎 | `[INSPECT]` | Inspect / look closer |
| 🧪 | `[LAB]` | Experimental trial |
| ✂️ | `[CLEAN]` | Cleanup / minimization / removal |
| 🔐 | `[APPROVE]` | Human sign-off |
| 🌙 | `[NIGHTSHIFT]` | Approved batch run |
| 📌 | `[PIN]` | Pin / defer a task |
| 🚫 | `[BLOCK]` | Blocked |
| ⏳ | `[SCHEDULE]` | Schedule a resource-intensive task |
| ⏰ | `[REVISIT]` | Revisit a pinned task |
| 📦 | `[PACKAGING]` | Bundle / ship an artifact |

## Rules

- Exactly one start-marker per unit — a paragraph, or a unit of work — placed at the start.
- Every marker may also close a unit (an end-marker).
- The word marker is mandatory, uppercase, and in square brackets: `[REPORT]`, never `REPORT:`.
- `[WARN]` is prepended to the unit it corrects, and stays active until the user resolves it.
- Ask to act, not ask to choose: a unit that asks the user to **permit an action** (write, delete, send, run) is `[APPROVE]`; a unit that asks the user to **choose an answer** is `[DECIDE]`. The line's punctuation never decides the marker.
- The start-marker semantics and the sibling order are defined by [parse-back](../parse-back/), not here.
- The set is closed and it is the chat vocabulary: entry-domain words (💡 `[LEARNING]` …) belong to the log's vocabulary ([evaluation-report](../evaluation-report/)) and never open a chat unit.
- No markers inside code blocks or inside table cells. The reference table above is the only exception.

## Anti-patterns

- `REPORT: text` — missing emoji and missing brackets.
- `🧾 REPORT text` — word marker not bracketed.
- `REPORT 🧾 text` — emoji placed after the word marker (wrong order).
- `note: text` — invented marker, wrong case.
- Emoji with no word marker while emoji are renderable.

## Synonyms

- "pin", "park", "defer" all mean `[PIN]`.
- "revisit", "resume", "remind" all mean `[REVISIT]`.
- "clean up", "tidy", "prune", "minify" all mean `[CLEAN]`.

## Not part of the protocol

Ad-hoc colored circles used only inside evaluation tables carry no marker: 🔴 (bad/blocked), 🟡 (medium), 🟢 (free/good).
