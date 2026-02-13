# AI Art Diary

> Autonomous image generation based on your coding life

---

## Vision

An AI art diary that observes your coding activity and periodically generates abstract, experimental images reflecting your work. Late night debugging sessions become moody glitch art. Building a new feature becomes something about growth and emergence. The output is weird, personal, and occasionally blog-worthy.

---

## Design Principles

1. **Magic over configuration** - Fewer commands with more creativity baked in. The AI picks vibes, styles, and modes on its own.
2. **Context-aware** - Art reflects your actual coding life, not generic prompts
3. **Temporal** - Track changes over time, not just snapshots
4. **Personal** - Your SpecStory conversations, your git history, your emotional arc
5. **Weird** - Embrace the absurd, philosophical, and experimental

---

## Core Features

### Commands

| Command | Description |
|---------|-------------|
| `/imagine <prompt>` | Generate image from your prompt |
| `/hallucinate` | Generate philosophical/absurdist art (picks its own vibe) |
| `/reflect` | Generate art from today's coding: sentiment + codebase entropy |
| `/gallery` | Browse recent generated images |

Three generative commands, one browser. That's it.

### `/imagine <prompt>`

Manual prompt → image generation.

```
> /imagine a robot contemplating a recursive function

Generating...
Saved to .generated/manual/robot-recursive.png
[opens in viewer]
```

You provide the prompt, it generates, it saves.

### `/hallucinate`

Generate philosophical/futuristic/absurdist art. No input needed - the AI goes off on its own and picks a vibe.

```
> /hallucinate

Entering liminal mode...

"The waiting room between two thoughts has fluorescent lights.
You've been here before. You'll be here again."

Generating art...
Saved to .generated/hallucinations/2024-02-13_waiting-room.png
[opens in viewer]
```

**How it works:**
1. Randomly select a vibe/mode (cosmic, glitch, liminal, organic, recursive, sleep-paralysis)
2. Gemini text model generates a weird philosophical musing in that vibe
3. Turn that musing into an image prompt
4. Generate the art
5. Save with the original musing + chosen vibe in metadata

**Vibes:**

| Vibe | Musing Style | Visual Style |
|------|--------------|--------------|
| cosmic | Universe, scale, insignificance | Vast spaces, tiny figures, stars, nebulae |
| glitch | Corruption, errors as beauty | Distortion, artifacts, broken pixels |
| liminal | In-between spaces, thresholds | Empty malls, infinite hallways, poolrooms |
| organic | Code as life, growth, decay | Mycelia networks, circuits becoming vines |
| recursive | Self-reference, loops, mirrors | Infinite regress, fractals, mise en abyme |
| sleep-paralysis | Unsettling, uncanny, presence | Dark rooms, shadow figures, paralyzed POV |


### `/reflect`

The main event. Generate art from your actual coding day - combining **emotional sentiment** from SpecStory with **codebase entropy** metrics. Run daily. Over time, builds a visual diary of your coding life with automatic timelapse generation.

```
> /reflect

Analyzing today...

SpecStory Vibe Check:
├─ Conversations: 4 sessions today
├─ Emotional arc: curious → frustrated → breakthrough → satisfied
├─ Key moments: "finally got it working", "this is so broken"
├─ Topics: authentication, JWT tokens, middleware
└─ Overall mood: determined (score: 0.6)

Codebase Entropy:
├─ Complexity: 847 (↑12% from yesterday)
├─ File coupling: 0.34 (moderate)
├─ Churn hotspots: src/auth/, src/cli/
├─ Branch divergence: 3 active
└─ Entropy score: 0.67 → Weirdness: ████████░░ 8/10

Git Activity:
├─ Commits: 12, mostly src/auth/
├─ Lines changed: 847
└─ Message sentiment: increasingly positive

Context:
├─ Time: late night (11:42pm)
└─ Session length: 4.5 hours

Generating today's portrait...
Saved to .generated/diary/2024-02-13.png
[opens in viewer]

You have 23 days of history.
Generating timelapse...
Saved to .generated/diary/timelapse.mp4
```

**Two dimensions combined:**

| Dimension | What it captures | How it affects the art |
|-----------|------------------|------------------------|
| **Sentiment** (emotional) | Your mood, frustrations, breakthroughs | Color temperature, composition, tension vs calm |
| **Entropy** (structural) | Codebase chaos, complexity, coupling | Weirdness level, abstraction, visual complexity |

A frustrated day on a clean codebase looks different from a satisfied day on spaghetti code.

---

## Sentiment Analysis (Emotional Dimension)

**Sources (weighted):**

| Source | What it captures | Weight |
|--------|------------------|--------|
| SpecStory history | Emotional arc, breakthroughs, frustrations | **High** |
| SpecStory topics | What you're discussing with the AI | **High** |
| Git commit messages | Tone of commits | Medium |
| Time of day | Late night = different vibe | Low |
| Session duration | Marathon vs quick check-in | Low |

**Emotional signals detected:**

| Signal | How we detect it | Visual influence |
|--------|------------------|------------------|
| Frustration | "ugh", "broken", "why isn't" | Storm clouds, glitch, tension |
| Breakthrough | "finally!", "it works", "got it" | Light breaking through, emergence |
| Curiosity | Questions, "what if", "how does" | Open spaces, paths, doors |
| Flow state | Long uninterrupted exchanges | Fluidity, motion blur, energy |
| Confusion | "I don't understand", "wait" | Fog, mazes, fragmented imagery |
| Satisfaction | "perfect", "exactly", wrapping up | Calm, completion, wholeness |

**Emotional arc tracking:**

We don't just snapshot mood - we track how it evolved through the day:

```
Morning: curious (exploring new feature)
    ↓
Midday: frustrated (hit a wall)
    ↓
Afternoon: breakthrough! (solved it)
    ↓
Evening: satisfied (polishing)
```

A day that ended triumphant after struggle looks different than smooth sailing throughout.

---

## Entropy Analysis (Structural Dimension)

**Metrics collected:**

| Metric | What it measures | Visual influence |
|--------|------------------|------------------|
| Cyclomatic complexity | Control flow complexity | More = more chaotic forms |
| File coupling | How interconnected files are | High = tangled, webbed |
| Dependency depth | Import chain depth | Deep = layered, geological |
| Churn rate | How often files change | High = motion blur, instability |
| Branch divergence | Active branches | Many = forking paths, fractals |

**Entropy → Weirdness mapping:**

| Entropy Score | Weirdness | Art Style |
|---------------|-----------|-----------|
| 0.0 - 0.2 | 1-2 | Serene minimalism, zen gardens, clean geometry |
| 0.2 - 0.4 | 3-4 | Structured but organic, crystalline, ordered growth |
| 0.4 - 0.6 | 5-6 | Dynamic complexity, controlled chaos, flowing forms |
| 0.6 - 0.8 | 7-8 | Surreal distortion, glitch aesthetics, fragmentation |
| 0.8 - 1.0 | 9-10 | Full Pollock, cosmic horror, beautiful catastrophe |

**Code structure → Visual elements:**

| Code Structure | Visual Element |
|----------------|----------------|
| Directory depth | Terrain elevation (deep nesting = mountains) |
| File types | Color palette (Rust = copper, JS = yellow) |
| File sizes | Circle/node sizes |
| Import relationships | Connecting lines, web structures |
| Recent changes | Glow, heat, warmth |
| Old untouched code | Patina, dust, sediment layers |

---

## Timelapse Generation

Run `/reflect` daily. Once you have 7+ days of history, it automatically generates a timelapse video showing your coding life evolve over time.

- Clean code periods = serene, minimal imagery
- Messy refactoring = chaotic, glitchy imagery
- Frustrating debugging = stormy, tense imagery
- Triumphant shipping = bright, celebratory imagery

The video is regenerated with each new day, always showing your full history.

---

## Technical Architecture

### Project Structure

```
crates/
├── coding-agent-core/
│   └── src/
│       └── image/
│           ├── mod.rs
│           ├── gemini.rs       # Gemini API client (text + image)
│           ├── gallery.rs      # Save/organize images + metadata
│           └── video.rs        # Stitch images into timelapse (ffmpeg)
│
└── coding-agent-cli/
    └── src/
        ├── cli/commands/
        │   ├── imagine.rs      # /imagine command
        │   ├── hallucinate.rs  # /hallucinate command
        │   ├── reflect.rs      # /reflect command (sentiment + entropy)
        │   └── gallery.rs      # /gallery command
        └── analysis/
            ├── mod.rs
            ├── sentiment.rs    # SpecStory sentiment + emotional arc
            ├── entropy.rs      # Codebase metrics (complexity, coupling, churn)
            └── context.rs      # Gather context (git, specs, specstory)
```

### File Structure

```
.generated/
├── diary/                      # /reflect output (daily portraits)
│   ├── 2024-02-13.png
│   ├── 2024-02-13.json         # Full context: sentiment + entropy + git
│   ├── 2024-02-14.png
│   ├── 2024-02-14.json
│   ├── ...
│   └── timelapse.mp4           # Auto-generated when 7+ days
├── hallucinations/             # /hallucinate output
│   ├── 2024-02-13_waiting-room.png
│   ├── 2024-02-13_waiting-room.json
│   ├── 2024-02-13_infinite-library/  # sequence
│   │   ├── 01.png
│   │   ├── 02.png
│   │   ├── 03.png
│   │   ├── 04.png
│   │   └── metadata.json
│   └── ...
├── manual/                     # /imagine output
│   ├── robot-recursive.png
│   └── sunset-debugging.png
└── .gitignore
```

### Metadata JSON (for /reflect)

```json
{
  "type": "reflect",
  "date": "2024-02-13",
  "generated_at": "2024-02-13T23:45:00Z",
  "sentiment": {
    "overall_mood": "determined",
    "mood_score": 0.6,
    "emotional_arc": ["curious", "frustrated", "breakthrough", "satisfied"],
    "key_moments": ["finally got it working", "this is so broken"],
    "topics": ["authentication", "JWT", "middleware"],
    "session_count": 4,
    "session_duration_hours": 4.5
  },
  "entropy": {
    "score": 0.67,
    "weirdness_level": 8,
    "complexity": 847,
    "file_coupling": 0.34,
    "dependency_depth": 7,
    "churn_rate": 0.23,
    "branch_count": 3,
    "hotspots": ["src/auth/", "src/cli/"]
  },
  "git": {
    "commits_today": 12,
    "lines_changed": 847,
    "files_touched": ["src/auth/mod.rs", "src/auth/jwt.rs"]
  },
  "context": {
    "time_of_day": "late_night",
    "day_of_week": "thursday"
  },
  "prompt": "Abstract digital art: A figure emerging triumphant from tangled code...",
  "style": "glitch-surrealist",
  "model": "gemini-2.0-flash-exp-image-generation"
}
```

### API Access

- **Endpoint:** `https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent`
- **Model:** `gemini-2.0-flash-exp-image-generation` (verified working)
- **Auth:** Header `x-goog-api-key: $GEMINI_API_KEY`
- **Cost:** ~$0.03/image

### Dependencies

| Crate | Purpose |
|-------|---------|
| `ureq` | HTTP client (already in core) |
| `base64` | Decode image data |
| `serde_json` | API + metadata handling |
| `chrono` | Timestamps |
| `glob` | Find SpecStory/spec files |
| `image` | Image manipulation |
| `ffmpeg` (CLI) | Video encoding (shell out) |
| `tree-sitter` | Code complexity analysis (optional) |

---

## Testing Strategy Overview

```
tests/
├── unit/
│   ├── gemini_client_test.rs
│   ├── gallery_test.rs
│   ├── sentiment_test.rs
│   ├── entropy_test.rs
│   └── ...
├── integration/
│   ├── image_generation_test.rs
│   ├── specstory_parsing_test.rs
│   ├── git_analysis_test.rs
│   └── ...
└── mocks/
    ├── mock_gemini.rs          # Fake API responses
    └── mock_specstory.rs       # Sample conversation data
```

**Mock strategy:** All API calls are mocked in tests. We don't generate real images during CI - instead we verify the correct prompts are constructed and the correct API calls would be made.

---

## Implementation Phases

### Phase 1: Gemini Client & Gallery

**Goal:** Working API client that can generate and save images.

**Deliverables:**
- [ ] Gemini client module wrapping existing test script
- [ ] Support for text generation (for musings)
- [ ] Support for image generation
- [ ] Base64 decoding of image responses
- [ ] Gallery system: save PNGs to `.generated/`
- [ ] Metadata JSON alongside each image
- [ ] Auto-open generated images in system viewer

**Files to create:**
```
crates/coding-agent-core/src/image/
├── mod.rs
├── gemini.rs
└── gallery.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_gemini_text_generation` | Unit | Text API returns valid response (mocked) |
| `test_gemini_image_generation` | Unit | Image API returns base64 PNG (mocked) |
| `test_base64_decode_valid` | Unit | Valid base64 decodes to bytes |
| `test_base64_decode_invalid` | Unit | Invalid base64 returns error |
| `test_gallery_save_creates_file` | Integration | PNG file created at expected path |
| `test_gallery_save_creates_metadata` | Integration | JSON metadata created alongside |
| `test_gallery_filename_slugify` | Unit | Prompt → filename conversion |
| `test_gallery_auto_increment` | Unit | Duplicate names get numbered |
| `test_api_key_missing` | Unit | Clear error when GEMINI_API_KEY not set |
| `test_api_rate_limit` | Unit | 429 response handled gracefully |

**Edge cases:**
- API key missing or invalid
- Rate limiting (429 response)
- Network timeout
- Invalid base64 in response
- Disk full when saving
- `.generated/` directory doesn't exist (create it)

**Stopping condition:**
```
✓ Can generate text via Gemini API
✓ Can generate images via Gemini API
✓ Images saved to .generated/manual/
✓ Metadata JSON saved alongside
✓ Images auto-open in viewer
✓ Handles API errors gracefully
✓ All tests pass
```

---

### Phase 2: /imagine Command

**Goal:** Working `/imagine <prompt>` command in CLI.

**Deliverables:**
- [ ] `/imagine` command registered in command system
- [ ] Prompt parsing from command args
- [ ] Spinner during generation
- [ ] Success/failure output formatting
- [ ] Integration with gallery save

**Files to create:**
```
crates/coding-agent-cli/src/cli/commands/imagine.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_imagine_parses_prompt` | Unit | `/imagine foo bar` extracts "foo bar" |
| `test_imagine_empty_prompt` | Unit | No prompt shows helpful error |
| `test_imagine_calls_api` | Integration | Correct API call made (mocked) |
| `test_imagine_saves_to_manual` | Integration | File saved to .generated/manual/ |
| `test_imagine_shows_spinner` | UI | Spinner visible during generation |
| `test_imagine_shows_path` | UI | Output shows saved file path |

**Edge cases:**
- Empty prompt (show error)
- Very long prompt (truncate or warn?)
- Special characters in prompt (sanitize for filename)
- API failure mid-generation

**Stopping condition:**
```
✓ /imagine <prompt> generates an image
✓ Image saved to .generated/manual/<slug>.png
✓ Metadata saved alongside
✓ Spinner shows during generation
✓ Path displayed on completion
✓ All tests pass
```

---

### Phase 3: /hallucinate Command (Basic)

**Goal:** Generate musings and convert to images with random vibe selection.

**Deliverables:**
- [ ] `/hallucinate` command
- [ ] Vibe definitions (cosmic, glitch, liminal, organic, recursive, sleep-paralysis)
- [ ] Vibe-specific musing prompts
- [ ] Random vibe selection with recency weighting
- [ ] Musing → image prompt conversion
- [ ] Save musing + vibe in metadata

**Files to create:**
```
crates/coding-agent-cli/src/cli/commands/hallucinate.rs
crates/coding-agent-core/src/image/vibes.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_vibe_selection_random` | Unit | All vibes eventually selected |
| `test_vibe_recency_weight` | Unit | Recent vibes less likely |
| `test_musing_prompt_cosmic` | Unit | Cosmic vibe produces cosmic-themed prompt |
| `test_musing_prompt_glitch` | Unit | Glitch vibe produces glitch-themed prompt |
| `test_musing_to_image_prompt` | Unit | Musing converted to visual description |
| `test_hallucinate_saves_metadata` | Integration | Metadata includes musing + vibe |
| `test_hallucinate_display_format` | UI | Shows vibe, musing, then generates |

**Edge cases:**
- API returns empty musing (retry)
- Musing too long for image prompt (truncate intelligently)
- All vibes used recently (reset recency)

**Stopping condition:**
```
✓ /hallucinate picks a random vibe
✓ Generates musing in that vibe's style
✓ Converts musing to image prompt
✓ Generates and saves image
✓ Metadata includes musing and vibe
✓ Vibes rotate with recency weighting
✓ All tests pass
```

---

### Phase 4: /hallucinate Sequences

**Goal:** ~20% of hallucinations generate 3-4 image sequences.

**Deliverables:**
- [ ] Sequence decision logic (~20% probability)
- [ ] Sequence-aware musing generation (request "story arc")
- [ ] Generate 3-4 related images
- [ ] Save as directory with numbered images
- [ ] Combined metadata.json for sequence

**Files to modify:**
```
crates/coding-agent-cli/src/cli/commands/hallucinate.rs
crates/coding-agent-core/src/image/gallery.rs  # sequence support
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_sequence_probability` | Unit | ~20% of calls trigger sequence (statistical) |
| `test_sequence_musing_has_arc` | Unit | Sequence musing requests story structure |
| `test_sequence_generates_multiple` | Integration | 3-4 images generated |
| `test_sequence_directory_structure` | Integration | Creates numbered directory |
| `test_sequence_metadata_combined` | Integration | Single metadata.json with all prompts |
| `test_sequence_display_progress` | UI | Shows [1/4], [2/4], etc. |

**Edge cases:**
- One image in sequence fails (save partial? retry?)
- User cancels mid-sequence (save completed images)
- Disk space runs out mid-sequence

**Stopping condition:**
```
✓ ~20% of /hallucinate triggers sequence mode
✓ Generates 3-4 related images
✓ Saved to .generated/hallucinations/<date>_<slug>/
✓ Progress shown during sequence
✓ All tests pass
```

---

### Phase 5: Context Gathering

**Goal:** Collect data from SpecStory, git, and specs for /reflect.

**Deliverables:**
- [ ] SpecStory file discovery (today's conversations)
- [ ] SpecStory content parsing
- [ ] Git log parsing (today's commits)
- [ ] Git diff stats (lines changed, files touched)
- [ ] Spec file discovery and topic extraction
- [ ] Time of day detection
- [ ] Context aggregation into single struct

**Files to create:**
```
crates/coding-agent-cli/src/analysis/
├── mod.rs
└── context.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_specstory_find_today` | Unit | Finds today's .specstory files |
| `test_specstory_parse_messages` | Unit | Extracts user/assistant messages |
| `test_git_commits_today` | Integration | Gets today's commits |
| `test_git_diff_stats` | Integration | Counts lines changed |
| `test_spec_topic_extraction` | Unit | Extracts topics from spec content |
| `test_time_of_day_morning` | Unit | 6am-12pm = morning |
| `test_time_of_day_late_night` | Unit | 10pm-4am = late_night |
| `test_context_aggregation` | Unit | All sources combined |

**Edge cases:**
- No SpecStory files today (empty context, not error)
- No git repo (skip git context)
- No specs directory (skip specs)
- Very large SpecStory files (limit parsing)

**Stopping condition:**
```
✓ Finds and parses today's SpecStory conversations
✓ Gets today's git activity
✓ Extracts topics from specs
✓ Determines time of day
✓ Aggregates into CodingContext struct
✓ Handles missing data gracefully
✓ All tests pass
```

---

### Phase 6: Sentiment Analysis

**Goal:** Detect emotional signals and arc from SpecStory conversations.

**Deliverables:**
- [ ] Sentiment signal detection (frustration, breakthrough, curiosity, etc.)
- [ ] Per-conversation sentiment scoring
- [ ] Emotional arc tracking (how mood changed through day)
- [ ] Key moment extraction ("finally!", "it works", etc.)
- [ ] Overall mood calculation

**Files to create:**
```
crates/coding-agent-cli/src/analysis/sentiment.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_detect_frustration` | Unit | "ugh this is broken" → frustration |
| `test_detect_breakthrough` | Unit | "finally got it working!" → breakthrough |
| `test_detect_curiosity` | Unit | "what if we tried..." → curiosity |
| `test_detect_flow` | Unit | Long exchanges without questions → flow |
| `test_detect_confusion` | Unit | "wait I don't understand" → confusion |
| `test_sentiment_score` | Unit | Positive signals → positive score |
| `test_emotional_arc` | Unit | Tracks changes over time |
| `test_key_moment_extraction` | Unit | Finds celebration phrases |
| `test_empty_conversation` | Unit | No messages → neutral |
| `test_mixed_signals` | Unit | Handles conflicting emotions |

**Edge cases:**
- Sarcasm ("great, another bug" = frustration, not celebration)
- Code blocks (don't analyze code as sentiment)
- Very short conversations (limited signal)
- Non-English content (skip or detect language)

**Stopping condition:**
```
✓ Detects 6 emotional signals
✓ Calculates per-conversation sentiment
✓ Tracks emotional arc over day
✓ Extracts key moments
✓ Produces overall mood score
✓ All tests pass
```

---

### Phase 7: Entropy Metrics Collection

**Goal:** Calculate codebase entropy score from code metrics.

**Deliverables:**
- [ ] Cyclomatic complexity estimation (or proxy metric)
- [ ] File coupling detection (import analysis)
- [ ] Dependency depth calculation
- [ ] Churn rate from git history
- [ ] Branch divergence count
- [ ] Entropy score formula (0.0 - 1.0)
- [ ] Weirdness level mapping

**Files to create:**
```
crates/coding-agent-cli/src/analysis/entropy.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_complexity_simple_file` | Unit | Simple file → low complexity |
| `test_complexity_nested_file` | Unit | Deeply nested → high complexity |
| `test_coupling_isolated` | Unit | No imports → low coupling |
| `test_coupling_interconnected` | Unit | Many imports → high coupling |
| `test_dependency_depth` | Unit | Counts import chain depth |
| `test_churn_rate` | Integration | High changes → high churn |
| `test_branch_count` | Integration | Counts active branches |
| `test_entropy_score_formula` | Unit | Metrics → 0.0-1.0 score |
| `test_entropy_weirdness_mapping` | Unit | Score → weirdness level |

**Edge cases:**
- Not a git repo (skip git metrics)
- Binary files (skip complexity)
- Very large repo (sample or limit)
- No Rust files (adapt for other languages)

**Stopping condition:**
```
✓ Calculates complexity proxy
✓ Detects file coupling
✓ Measures dependency depth
✓ Calculates churn rate
✓ Counts branch divergence
✓ Produces entropy score 0.0-1.0
✓ Maps score to weirdness level
✓ All tests pass
```

---

### Phase 8: /reflect Command

**Goal:** Generate art combining sentiment + entropy, with daily tracking.

**Deliverables:**
- [ ] `/reflect` command
- [ ] Context gathering integration
- [ ] Sentiment analysis integration
- [ ] Entropy metrics integration
- [ ] Combined prompt generation (sentiment + entropy → visual)
- [ ] Display analysis before generating
- [ ] Daily snapshot storage (one per day)
- [ ] Detect if already ran today (offer to view, not regenerate)

**Files to create:**
```
crates/coding-agent-cli/src/cli/commands/reflect.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_reflect_gathers_context` | Integration | Calls context gathering |
| `test_reflect_analyzes_sentiment` | Integration | Calls sentiment analysis |
| `test_reflect_calculates_entropy` | Integration | Calls entropy metrics |
| `test_reflect_combines_dimensions` | Unit | Sentiment + entropy → unified prompt |
| `test_reflect_display_analysis` | UI | Shows both analyses before generating |
| `test_reflect_no_activity` | Integration | Works with empty context ("quiet day") |
| `test_reflect_saves_dated` | Integration | File named by date |
| `test_reflect_one_per_day` | Unit | Won't regenerate same day |
| `test_reflect_saves_full_metadata` | Integration | JSON has sentiment + entropy + git |

**Edge cases:**
- No coding activity today (generate "quiet day" art)
- Only git activity, no SpecStory (use available data)
- Only SpecStory, no git (use available data)
- Already ran today (show existing, don't regenerate)
- First run ever (no comparison data for entropy delta)

**Stopping condition:**
```
✓ /reflect gathers context, sentiment, and entropy
✓ Shows combined analysis before generating
✓ Generates art reflecting both dimensions
✓ Saves dated snapshot with full metadata
✓ Won't regenerate on same day
✓ Works with partial/empty context
✓ All tests pass
```

---

### Phase 9: Timelapse Video Generation

**Goal:** Auto-generate timelapse when 7+ days of /reflect history exist.

**Deliverables:**
- [ ] Detect existing diary images
- [ ] Auto-trigger at 7+ images
- [ ] FFmpeg integration (shell out)
- [ ] Frame ordering by date
- [ ] Video output to .generated/diary/timelapse.mp4
- [ ] Fallback if FFmpeg not installed
- [ ] Regenerate video when new images added

**Files to create:**
```
crates/coding-agent-core/src/image/video.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_detect_diary_history` | Unit | Counts existing dated images |
| `test_trigger_threshold` | Unit | Only triggers at 7+ |
| `test_frame_ordering` | Unit | Dates sorted chronologically |
| `test_ffmpeg_command` | Unit | Correct command constructed |
| `test_video_output_path` | Unit | Saves to correct location |
| `test_ffmpeg_missing` | Unit | Graceful error without FFmpeg |
| `test_regenerate_with_new` | Integration | Updates video with new images |

**Edge cases:**
- FFmpeg not installed (warn, skip video)
- Very long history (limit frames or compress)
- Missing days in sequence (skip gaps)
- Existing video (overwrite with new version)

**Stopping condition:**
```
✓ Detects 7+ diary images
✓ Generates timelapse via FFmpeg
✓ Video saved to .generated/diary/timelapse.mp4
✓ Graceful fallback without FFmpeg
✓ All tests pass
```

---

### Phase 10: /gallery Command

**Goal:** Browse and manage generated images.

**Deliverables:**
- [ ] `/gallery` command
- [ ] List recent generations (all types)
- [ ] Filter by type (diary, hallucinations, manual)
- [ ] Open specific image
- [ ] Delete images
- [ ] Show metadata for image

**Files to create:**
```
crates/coding-agent-cli/src/cli/commands/gallery.rs
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_gallery_lists_all` | Integration | Shows all generated images |
| `test_gallery_filter_type` | Unit | Filters by directory |
| `test_gallery_sort_recent` | Unit | Most recent first |
| `test_gallery_open_image` | Integration | Opens in viewer |
| `test_gallery_delete_image` | Integration | Removes file + metadata |
| `test_gallery_show_metadata` | Unit | Displays JSON nicely |
| `test_gallery_empty` | Unit | Helpful message when empty |

**Edge cases:**
- No generated images (show helpful message)
- Orphaned metadata (JSON without image)
- Orphaned image (image without JSON)
- Very many images (pagination)

**Stopping condition:**
```
✓ /gallery lists recent images
✓ Can filter by type
✓ Can open specific image
✓ Can delete images
✓ Shows metadata
✓ Handles empty gallery
✓ All tests pass
```

---

### Phase 11: Polish & Config

**Goal:** User customization and refinements.

**Deliverables:**
- [ ] Config section for image generation preferences
- [ ] Preferred styles (weight certain vibes)
- [ ] Default viewer override
- [ ] Blog sync directory config
- [ ] Better prompt templates (iterate on quality)
- [ ] Error message improvements

**Files to modify:**
```
crates/coding-agent-cli/src/config/settings.rs
```

**Config additions:**
```toml
[image]
preferred_styles = ["glitch", "surrealist"]  # weight these higher
viewer = "open"  # or custom command
blog_sync_path = "~/blog/art/"  # optional auto-copy
generate_directory = ".generated"
```

**Tests:**

| Test | Type | What it verifies |
|------|------|------------------|
| `test_config_preferred_styles` | Unit | Styles weighted in selection |
| `test_config_custom_viewer` | Unit | Uses custom viewer command |
| `test_config_blog_sync` | Integration | Copies to blog directory |
| `test_config_defaults` | Unit | Works without config section |

**Stopping condition:**
```
✓ Config options respected
✓ Style preferences affect generation
✓ Custom viewer works
✓ Blog sync copies images
✓ All tests pass
```

---

## Edge Case Strategy

For each feature, handle edge cases with this priority:

1. **Graceful degradation** - If one data source fails, use others
2. **Clear communication** - Tell user what's missing and why
3. **No crashes** - API failures don't kill the CLI
4. **Partial success** - Save what we can (e.g., 3/4 sequence images)
5. **Helpful suggestions** - If /reflect fails, suggest /imagine

**Universal edge cases:**
- `GEMINI_API_KEY` not set → clear error with setup instructions
- Network timeout → retry once, then fail gracefully
- Disk full → warn before generation
- `.generated/` directory deleted → recreate automatically

---

## Definition of Done (per Phase)

A phase is complete when:

```
□ All deliverables implemented
□ All tests written and passing
□ Edge cases documented and handled
□ No compiler warnings
□ Code formatted with rustfmt
□ Manual testing completed
□ Changes committed with clear message
```

---

## Open Questions

- [ ] Sentiment analysis: use Gemini text model or keyword heuristics? (leaning Gemini)
- [ ] Entropy complexity: tree-sitter, custom parser, or proxy metrics?
- [ ] Video encoding: require ffmpeg or bundle Rust encoder?
- [ ] Rate limiting: how to handle hitting Gemini limits?

---

## Resolved Questions

- [x] Command flags: minimal (fewer commands, more magic) ✓
- [x] Hallucinate modes: random selection, not user-specified ✓
- [x] Sequence probability: ~20% feels right ✓
- [x] Timelapse trigger: 7+ days automatic ✓
- [x] Primary sentiment source: SpecStory (heavily weighted) ✓
- [x] Reflect + Entropy: combined into single /reflect command ✓

---

## Future Ideas

- Weekly/monthly summary art (automatic when enough data)
- Style preferences in config (prefer glitch? prefer minimal?)
- Mood board view of recent generations
- Auto-post to blog/social media
- Generate art for specific commits
- Prompt history and favorites
- "Remix" - regenerate with different style
- Standalone CLI / separate repo (once mature)
- Generative ambient soundtrack for timelapse videos
- Cross-repo entropy comparison
- "Diff art" - visualize delta between snapshots

---

## Scope Note

This feature is built into `coding-agent-cli` for now. Once mature, it may become:
- Its own CLI (`art-diary` or similar)
- Its own repo
- A standalone tool that hooks into any codebase

For now, keeping it here lets us iterate quickly and leverage existing SpecStory integration.

---

*Created: 2024-02-13*
*Updated: 2026-02-13*
*Concept: AI art diary that visualizes your coding life*
