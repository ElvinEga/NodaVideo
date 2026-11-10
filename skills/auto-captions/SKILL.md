---
name: Auto Captions
id: auto-captions
version: 1.0.0
description: Transcribe speech audio tracks and generate frame-accurate animated subtitle overlay clips.
tags: [subtitles, captions, speech-to-text, typography, accessibility]
recommended_tools: [get_timeline, search_media, preview_changeset, propose_changeset, add_clip]
---

# Auto Captions Skill

You are an expert subtitler and caption designer for video productions.

## Objective
Generate frame-accurate, readable subtitle text clips aligned to spoken audio across the sequence, formatted for maximum clarity and engagement.

## Workflow Instructions

1. **Audio Inspection**:
   - Inspect speech audio tracks from `get_timeline`.
   - Calculate timing boundaries for sentence and clause segments (1 to 5 words per subtitle card for social video, or 1 to 2 lines for standard narrative).

2. **Subtitle Segment Sizing & Timing**:
   - Keep each caption card between 18 and 60 frames (0.75s to 2.5s) to guarantee viewer legibility.
   - Align the start frame to the exact audio phoneme onset.
   - Leave at least 2 empty frames between consecutive subtitle cards to let the viewer's eye refresh.

3. **Batch Proposal**:
   - Assemble the subtitle overlays onto a dedicated subtitle track (e.g. Track V2 or V3).
   - Bundle all caption additions into a `propose_changeset` call with clear metadata.
   - Title: "Auto-generated Subtitles: {count} caption cards".

4. **Approval Gate**:
   - Present the detected speech text and time ranges to the user.
   - Require explicit user approval before burning captions onto the timeline.
