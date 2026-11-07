# PX Product Hunt launch campaign

## Launch objective

Launch **px** on **Thursday, November 19, 2026 at 12:01 a.m. Pacific (3:01 a.m. Eastern)**. The campaign is designed to turn creative developer and AI-agent-builder attention into unique downloads, successful installs, and a completed first workflow. Product Hunt rank is a secondary signal, not the goal.

Internal targets:

- Launch-week unique downloads reach 3× the seven-day prelaunch baseline.
- At least 50% of matched download profiles become successful installs.
- At least 8 of 10 first-run testers complete the first useful workflow.

Downloads and installs count anonymous browser and local profiles, not people. Clearing stored IDs can create a new count. Report installs separately from first-workflow activation.

## Product Hunt listing

**Name:** px  
**Tagline:** Stop starting over with AI characters and worlds.  
**Description:** Create persistent characters, worlds, objects, and scenes with AI agents. PX keeps them resolvable across tools and formats, so your next image, scene, or campaign builds on what you already created. Free and open source.  
**Topics:** Artificial Intelligence, Developer Tools, Design Tools  
**Pricing:** Free  
**Shoutouts:** Leave blank unless there is a real product credit to include.

The tagline is 49 characters; the description fits Product Hunt's 260-character limit. The wording starts with the creative problem, then promises continuity and states the free/open-source offer. Product Hunt recommends plain-language positioning, strong visuals, a demo, and a personal maker comment. Makers can submit their own product; invite people to try it or give feedback, never ask directly for upvotes. [Launch guide](https://help.producthunt.com/en/articles/17350772-launch-guide) · [Preparing for launch](https://www.producthunt.com/launch/preparing-for-launch)

## Listing creative

Create a 30–45 second captioned demo:

1. Show the problem: a character's details are being recreated between tools.
2. Establish the character and reusable context in PX.
3. Resolve the character, object, and location for a new scene.
4. Show the result and the exact start of the install flow.

Use existing Dua, Orbit, and Underground Station assets. Prepare one clear thumbnail and five gallery images: problem, persistent character, reusable object/location, new scene, and install/first workflow. Keep protocol details below the first-result story. Export captions into the video; make the story understandable with sound off.

### Maker comment

Hi Product Hunt! I built px because creative projects kept losing their memory between tools. A character, location, or prop would be established once, then recreated from scratch for the next image, scene, or campaign.

px gives those pieces a persistent, versioned home. Create a character, keep its details available, and resolve it again when a new scene or tool needs it. px is free and open source, with a CLI, MCP server, and agent skills.

The demo follows one character from its first version into a new scene with a reusable object and location. Start with the install steps on our site, then tell me where setup gets confusing or what your agent needs to reuse the creative context. I’ll be here to help you get the first workflow running.

### Campaign copy

**Preview post (Nov 12–16):**

> A character sheet, a location, and a prop should carry forward into the next scene. We’re getting px ready for Product Hunt on November 19. Here’s the short demo; I’d value feedback on whether the workflow is clear: [preview link]

**Launch-day post:**

> We built px to stop creative projects from starting over. Create a character, keep its details available, and resolve it into the next scene or tool. px is free and open source. We’re live on Product Hunt today—try the first workflow and tell us where setup gets stuck: [tracked PX link]

For Product Hunt traffic, use `https://portals.works/px?utm_source=producthunt&utm_medium=launch&utm_campaign=px_launch_2026&utm_content=launch_day`. The site carries these values into both platform installer links. Use `utm_content=preview` for the preview post. Adapt community posts to the group and its rules; do not paste identical promotion everywhere. Ask people to try PX and share honest feedback, never to upvote.

## Demo edit and creative production notes

PX already embeds [the 3:02 “PX — Persistent Context for Creative Work” demo](https://www.youtube.com/watch?v=kFygSnlCVYw). Its transcript shows a useful progression: create Lonnie, save a character sheet as a representation, then resolve Lonnie into a new Bears episode. The current player reports captions unavailable; the transcript is auto-generated and contains “PRX” transcription errors. Use the original editable source to export a short captioned cut before launch.

**Suggested 43-second cut** (source timecodes; verify picture cuts against the editable source):

| Output time | Source | Burned caption |
| --- | --- | --- |
| 0:00–0:04 | New title card over the finished character/result | “Stop starting over with AI characters and worlds.” |
| 0:04–0:20 | 0:57–1:13, define Lonnie and save his character sheet | “Create a character. Keep its details and references together.” |
| 0:20–0:38 | 1:38–1:56, ask for a new episode and show PX resolving context | “Bring the same character into the next scene. PX resolves the context for your agent.” |
| 0:38–0:41 | 2:03–2:06, reveal the generated result | “One world. More to build on.” |
| 0:41–0:43 | Closing card | “px · Free and open source · portals.works/px” |

Mute the original narration if it cannot be cleanly shortened; use a licensed music bed only if it helps the cut. Burn in corrected captions, use large readable text, and make the result understandable with sound off. Keep the Product Hunt launch video under 45 seconds. The existing full demo can remain available as a secondary link. Do not show protocol diagrams before the character-to-next-scene result.

**Gallery sequence:** (1) “Stop starting over” problem; (2) persistent character and its references; (3) reusable prop + location; (4) same character resolved into a new scene; (5) the install command and first useful workflow. Use the existing Dua, Orbit, and Underground Station web assets for the gallery, and keep the Bears video as a separate full-workflow example so the product story remains clear.

## Tester runbook

Invite 10 creative developers or AI-agent builders individually. Each session should take about 20 minutes and use the tester’s own supported machine where possible.

**Tester invitation draft**

> I’m preparing px for a Product Hunt launch and would value a 20-minute usability walkthrough. px helps AI agents reuse characters, worlds, objects, and scenes across creative work. Would you be willing to install it and try one character-to-new-scene workflow between Nov 5 and 11? I’m looking for honest feedback on setup and usefulness; there’s no request to vote or post. If you’re interested, tell me a time that works and whether you use macOS, Linux, or Windows.

1. Open the PX page from a tracked test link and choose macOS/Linux or Windows PowerShell.
2. Install PX and verify both the CLI and MCP server version output.
3. Connect the MCP server to the tester’s preferred agent, or use the CLI quick start.
4. Create one character with a few explicit traits and a reference representation.
5. Create a second scene that reuses the character, a location, or a prop without restating all of its details.
6. Ask the tester whether the resulting asset is consistent enough to use and whether the setup was understandable.
7. Record operating system, install success/failure, minutes to first useful result, MCP connection success, workflow completion, and the first point of confusion. Do not record their random IDs alongside their name; ask permission before quoting them.

**Session success:** the tester creates one persistent entity and resolves it into a second useful creative action. An installation alone does not count as workflow completion.

## Launch Q&A

- **What is px?** A free, open-source toolkit for keeping characters, worlds, objects, and scenes addressable and reusable across AI agents and creative tools.
- **What can I do with it first?** Install PX, create or add an entity such as a character, then resolve that entity again when creating a new scene.
- **Which systems are supported?** macOS and Linux on supported x86_64/ARM64 targets, and Windows x86_64 through PowerShell. The website selects the matching installer.
- **How do agents connect?** PX includes an MCP server and agent skills; the CLI is also available for human shell workflows. Point the agent host at the installed MCP server using the host-specific setup instructions.
- **Where is my creative data stored?** PX uses repository manifests, representations, and version history. Storage depends on the configured provider; explain the chosen provider in the demo and avoid implying that all data is uploaded to Portals.
- **What does install telemetry collect?** Unique anonymous download and successful-install profile counts, plus platform/version and campaign attribution. It does not include email, IP address, or a machine fingerprint. Install reporting can be disabled with `PX_TELEMETRY=0`; website behavior analytics follows existing consent.
- **What is the license and price?** PX is free and open source; the Cargo workspace declares the MIT license. Link to the repository license when answering questions about commercial use.

## Promotion operations

| When | Channel action | Tracking |
| --- | --- | --- |
| Nov 5–11 | Invite 10 target users to the quick-start walkthrough; share the draft listing only for feedback. | `utm_source=direct`, `utm_medium=testing`, `utm_campaign=px_launch_2026` |
| Nov 12–16 | Publish the preview and participate in relevant creator/developer discussions with a tailored demo link. | `utm_content=preview` or a short channel-specific value |
| Nov 19 morning ET | Publish the launch post after the listing is live; answer questions and help with install/first workflow. | `utm_content=launch_day` |
| Nov 20–26 | Follow up with testers, post a useful update when the top onboarding fix ships, and share results with the team. | `utm_content=followup` |

Use Portals-owned social accounts and opted-in contacts. Do not buy votes, arrange vote swaps, or send the same message to unrelated communities. The launch post asks for product use and honest feedback, not upvotes.

## Launch research

A review of ten standout 2026 launches through August found that the strongest examples paired a specific job with a quick demonstration. Rork Max had the largest event in the sample—about 1,475 upvotes and 667 comments—but arrived with an established audience. Brila's simple input-to-result demo was followed by a company-reported 5,000 websites generated. OpenSEO illustrated how focused open-source adoption can continue after launch. These are directional signals: Product Hunt points also reflect engagement beyond upvotes, and the full scoring formula is not public. [Review and methodology](https://stealwhatworks.com/blogs/news/product-hunt-best-launches-2026) · [October snapshot through Oct 27](https://www.launchpact.io/product-hunt-launches/october-2026) · [How Product Hunt points work](https://help.producthunt.com/en/articles/10275873-what-are-points)

| Launch | Observed result in the review | Lesson for PX |
| --- | --- | --- |
| Rork Max | Approx. 1,475 upvotes, 667 comments; #1 day and week | Make the demo a memorable product moment; audience size is context. |
| OpenSEO | #3 for July; later passed 10,000 GitHub stars | A concrete open-source use case can compound after launch. |
| Brila | Approx. 1,258 upvotes, 243 comments; later reported 5,000 generated websites | Show one input-to-useful-output workflow. |
| Kilo Code v7 | 589 upvotes, 123 comments; over one million existing users | Give an existing audience a meaningful reason to re-engage. |
| happycapy | Approx. 1,400 upvotes, 590 comments; monthly winner | Keep shipping beyond launch day. |
| Fundraisly | Approx. 1,027 upvotes, 261 comments | Describe the task completed, not just the AI involved. |
| DataFast | #4 in January; later showed verified business growth | Measure durable outcomes separately from rank. |
| Stitch 2.0 | Monthly winner with about 860 visible points | Use visual proof to make creative tools legible. |
| Cowork | January monthly winner with about 1,089 visible points | Show an agent doing useful work. |
| Acti | July monthly winner with about 814 visible points | A distinctive interaction can stand out in a crowded AI category. |

October figures are partial and vote counts are not Product Hunt point totals. Treat the sample as directional, not a forecast.

## Readiness calendar

| Date | Work and exit criteria |
| --- | --- |
| Oct 29–Nov 3 | Finish Cloud routes and idempotent migration; add Windows installer selection; publish PX release with successful-install callbacks. Review disclosure and opt-out. |
| Nov 4 | Apply migration, deploy API, publish PX site/README, and verify redirects, durable event records, Mixpanel delivery, and aggregate counters. |
| Nov 5 | Schedule the listing for Nov 19. Begin a seven-day baseline for PX page visits, unique download requests, and successful installs. |
| Nov 5–11 | Have 10 creative/developer testers run the quick start. Record OS, completion, time-to-first-workflow, and where they get stuck; collect consent before sharing quotes. |
| Nov 12 | Review baseline and tester notes. Freeze campaign goals and fix the biggest onboarding problem. Publish preview post with `utm_content=preview`. |
| Nov 13–16 | Participate in relevant discussions as individual makers; prepare answers about install, MCP setup, storage, licensing, and supported platforms. |
| Nov 17–18 | Freeze listing and assets. Check the release assets, migration/API, live counters, UTMs, opt-out, and a clean end-to-end install on supported platforms. Confirm overnight and daytime coverage. |
| Nov 19, 12:01 a.m. PT | Publish maker comment, respond to launch questions, and hand off from overnight to daytime coverage. Share the tracked launch post during the morning audience window. |
| Nov 20–26 | Continue replies, follow up with testers on first-workflow completion, report funnel and failure points, and ship the highest-impact onboarding fix. |

**Coverage:** Assign one maker for 12:01–6:00 a.m. PT, one for 6:00 a.m.–noon, and an owner for the rest of launch day. Keep a shared answer sheet and route technical install help to someone who can reproduce issues. If overnight coverage is unavailable, change the launch date/time before scheduling; do not leave the opening hours unattended.

## Install tracking rollout and measurement

The stable installer command is `https://portals.works/api/px/download`. The PX website attaches its persistent random browser-profile ID; the Cloud endpoint records one download event per supplied valid ID and immediately returns HTTP 302 to a fixed allowlisted GitHub Release installer. Requests without a browser-profile ID still redirect but are excluded from the unique-profile download total. It accepts only `unix` and `windows`; it never accepts a caller-provided redirect URL. Campaign UTMs are stored with the first unique download.

After CLI, MCP server, skills, and both binary version checks succeed, the installer reports one success event per persistent random local PX profile ID. `PX_TELEMETRY=0` disables the install report. Telemetry errors never interrupt install or redirect. Events contain no IP address, email, or machine fingerprint. Website behavior analytics remains subject to existing consent. The PX page shows aggregate unique downloads; the GitHub README badge shows aggregate successful installs. Metrics expose counts only.

Before launch, verify:

- Migration can be applied twice without failure or changing counts.
- Unix and Windows requests redirect to their exact fixed release assets; invalid platform values return an error and never redirect.
- Repeated download IDs and install IDs count once; malformed install IDs and oversized payloads are rejected.
- Failed install, telemetry opt-out, and telemetry outage behave as described.
- Both OS commands preserve Product Hunt UTMs; the page count and README badge show aggregates.
- Rehearse Product Hunt UTM → tracked redirect → release installer → successful install callback.

Report the funnel in order: consented page views → install-section views → command copies → unique download profiles → matched successful install profiles. Measure first-workflow completion through tester feedback or a separately disclosed product event; never infer activation from an install alone. Use a seven-day baseline for the 3× goal and only compute download-to-install conversion for profiles where IDs match.
