---
title: "tell me about the context tool bar"
created: 2026-02-14T03:01:31Z
updated: 2026-02-14T03:02:06Z
model: claude-3-opus
version: 1
---

# tell me about the context tool bar

## User

tell me about the context tool bar

## Agent

I don't see any files or code in the current directory that would give me information about a "context tool bar." Could you provide more details about what you're referring to? 

Let me first check what files are available in the current directory:

## Agent

Now let me search for references to "context tool bar" or similar terms in the codebase:

## Agent

Found it! Let me read the actual context bar implementation to give you a comprehensive overview:

## Agent

Now let me also check the specification to get a complete picture:

## Agent

Based on my exploration of this codebase, I can tell you about the **context bar** - it's a visual indicator at the bottom of the CLI that shows AI context window usage.

## Context Bar Overview

The context bar is a comprehensive token usage visualization system with these key features:

### **Visual Display**
- **Position:** Fixed at the bottom of the terminal
- **Progress bar:** Shows filled vs. empty segments (e.g., "████████████░░░░░░░░░░░░░░░░░░")
- **Percentage:** Current usage like "38% used"  
- **Token counts:** Current vs. max tokens like "76k / 200k tokens"

### **Color Coding**
The bar changes color based on usage levels:
- **Green:** 0-60% usage (safe)
- **Yellow:** 60-85% usage (caution) 
- **Red:** 85%+ usage (approaching limit)

### **Real-time Updates**
- Updates after each message exchange
- Tracks cumulative tokens for the entire session
- Uses `tiktoken-rs` for accurate token counting

### **Configuration Options**
```toml
[behavior]
show_context_bar = true  # Enable/disable
```

### **Example Output**
```
Context: [████████████░░░░░░░░░░░░░░░░░░]  38% used | 76k / 200k tokens
```

The context bar helps you monitor how much of Claude's context window you've used up, which is crucial for managing long conversations and understanding potential costs.

There's also a compact version for narrow terminals and integration with the `/cost` command for detailed breakdowns.

