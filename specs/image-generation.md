# Image Generation Experiments

> Creative/experimental image generation integrated into the coding agent

---

## Vision

Use Gemini's Imagen 3 to generate art, visualizations, and hallucinations based on code, conversations, and errors. The goal is exploration and delight, not utility.

---

## API Access

- **Provider:** Google Gemini API
- **Endpoint:** `https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent`
- **Auth:** Header `x-goog-api-key: $GEMINI_API_KEY`
- **Cost:** ~$0.03/image

### Available Models

| Model | Name | Best For |
|-------|------|----------|
| `gemini-2.0-flash-exp-image-generation` | Nano Banana (experimental) | **Verified working** |
| `gemini-2.5-flash-image` | Nano Banana (stable) | Production use |
| `gemini-3-pro-image-preview` | Nano Banana Pro | High quality, 4K support |

### Request Format

```json
{
  "contents": [{
    "parts": [{"text": "your prompt here"}]
  }],
  "generationConfig": {
    "responseModalities": ["TEXT", "IMAGE"],
    "imageConfig": {
      "aspectRatio": "1:1",
      "imageSize": "1K"
    }
  }
}
```

### Response Format

Images returned as base64 PNG in `candidates[0].content.parts[].inlineData.data`

### Test Script

```bash
cargo run -p coding-agent-core --example test_gemini_image
```

---

## Experiments to Build

### 1. Diff Paintings

Generate abstract art from git diffs.

**How it works:**
1. Parse `git diff` output
2. Extract metrics: lines added/removed, files changed, change types
3. Build evocative prompt based on the "feel" of the change
4. Generate image
5. Save alongside commit or display in terminal

**Prompt mapping ideas:**
| Diff characteristic | Visual metaphor |
|---------------------|-----------------|
| Many additions | Growth, bloom, expansion, green tendrils |
| Many deletions | Decay, pruning, minimalism, clean space |
| Refactoring (similar +/-) | Transformation, metamorphosis, seasons changing |
| New file | Birth, sunrise, blank canvas coming to life |
| Deleted file | Sunset, farewell, door closing |
| Bug fix | Healing, mending, light breaking through |
| Large change | Storm, earthquake, revolution |
| Small change | Ripple, whisper, single brushstroke |

**Output:** PNG saved to `.generated/diffs/` or displayed as ASCII

---

### 2. ASCII Hallucinations

Generate images and render them as colored ANSI art in the terminal.

**How it works:**
1. Generate image from prompt
2. Download/receive image bytes
3. Convert to ASCII using block characters and ANSI colors
4. Display in terminal

**Use cases:**
- `/imagine <prompt>` command - generate and display inline
- Loading screens during long operations
- Session artwork

**Technical approach:**
- Use half-block characters (`▀`, `▄`, `█`) for pseudo-pixels
- Map image colors to nearest ANSI 256 or truecolor
- Scale image to terminal width

---

### 3. Bug Bestiary

Generate monster/creature art for errors and bugs.

**How it works:**
1. Catch an error (compile error, runtime panic, test failure)
2. Parse error type and message
3. Generate creature prompt based on error characteristics
4. Build a "bestiary" collection over time

**Error → Creature mapping:**
| Error type | Creature style |
|------------|----------------|
| `cannot find crate` | Lost wandering spirit, searching |
| `type mismatch` | Chimera, hybrid beast |
| `borrow checker` | Possessive dragon guarding treasure |
| `stack overflow` | Ouroboros, infinite serpent |
| `null pointer` | Ghost, void creature |
| `timeout` | Sloth, frozen in time |
| `syntax error` | Babel fish, garbled creature |

**Output:** Saved to `.generated/bestiary/` with error hash as filename

---

### 4. Session Postcards

Generate a "postcard" image summarizing a coding session.

**How it works:**
1. At session end (or via `/postcard` command)
2. Analyze conversation: topics discussed, files touched, mood
3. Generate scenic/artistic image representing the session
4. Save with session metadata

**Prompt ingredients:**
- Main topics/themes from conversation
- Time of day (morning = sunrise vibes, night = starry)
- Success/frustration ratio
- Types of work (debugging = detective noir, feature = construction)

---

### 5. Code Tarot

Generate tarot-style cards for code patterns.

**Cards to generate:**
- The Recursive Function (infinity, spirals)
- The Null Pointer (void, absence)
- The Legacy Code (ancient ruins, cobwebs)
- The Race Condition (two forces colliding)
- The Memory Leak (overflowing vessel)
- The Beautiful Abstraction (crystal, perfect geometry)
- The TODO Comment (unfinished bridge)
- The Unit Test (shield, armor)

**Use:** Random card on startup, or generate based on code analysis

---

## Technical Architecture

```
src/
├── image/
│   ├── mod.rs
│   ├── gemini.rs        # Gemini API client for Imagen
│   ├── ascii.rs         # Image → ASCII/ANSI conversion
│   ├── prompts.rs       # Prompt generation from code/diffs/errors
│   └── gallery.rs       # Save/organize generated images
├── cli/commands/
│   ├── imagine.rs       # /imagine command
│   └── postcard.rs      # /postcard command
```

### Gemini API Integration

```rust
// Pseudo-code for Imagen 3 API call
struct GeminiImageClient {
    api_key: String,
}

impl GeminiImageClient {
    async fn generate(&self, prompt: &str) -> Result<Vec<u8>, Error> {
        // POST to Gemini imagegeneration endpoint
        // Return image bytes
    }
}
```

### ASCII Conversion

```rust
fn image_to_ascii(image: &[u8], width: usize) -> String {
    // 1. Decode image (PNG/JPEG)
    // 2. Resize to terminal width
    // 3. For each 2-pixel vertical pair:
    //    - Get top and bottom colors
    //    - Output ▀ with fg=top, bg=bottom
    // 4. Return ANSI-colored string
}
```

---

## Commands

| Command | Description |
|---------|-------------|
| `/imagine <prompt>` | Generate image and display as ASCII |
| `/postcard` | Generate session summary image |
| `/bestiary` | View collected bug creatures |
| `/diffart` | Generate art from current uncommitted changes |

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client for Gemini API |
| `image` | Image decoding and manipulation |
| `base64` | Encode/decode image data |
| `serde_json` | API request/response handling |

---

## Open Questions

- [x] Exact Gemini Imagen 3 API format (need to verify endpoint) → **Verified, see API Access section**
- [ ] Image size/quality tradeoffs for terminal display
- [ ] Store images in `.generated/` or `.specstory/images/`? → **Leaning `.generated/`**
- [ ] Rate limiting / cost awareness?
- [ ] Should ASCII art be cached?
- [ ] Fallback to free APIs (Pixazo, Flux Schnell) if Gemini fails?

---

## Implementation Order

1. **Gemini client** - Get basic image generation working
2. **ASCII conversion** - Display images in terminal
3. **`/imagine` command** - Interactive generation
4. **Diff paintings** - Integrate with git
5. **Bug bestiary** - Hook into error handling
6. **Session postcards** - End-of-session generation
7. **Code tarot** - Fun bonus

---

*Created: 2025-02-13*
