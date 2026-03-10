---
name: flowleap-chat
version: 1.0.0
description: "FlowLeap Chat: AI chat completions with SSE streaming."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap chat --help"
---

# FlowLeap Chat

Prerequisite: Read `flowleap-shared` for authentication and global flags.

## Usage

```bash
flowleap chat <message> [flags]
```

Posts to `/v1/chat/completions` (OpenAI-compatible endpoint) with SSE streaming by default.

## Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--model`, `-m` | Model to use | `patent-gemini-3-flash` |
| `--system` | System prompt | — |
| `--no-stream` | Wait for full response (no streaming) | `false` |

## Examples

```bash
# Basic chat
flowleap chat "What is claim 1 of EP1234567?"

# With specific model
flowleap chat -m patent-claude-sonnet "Summarize this patent's novelty"

# With system prompt
flowleap chat --system "You are a patent attorney" "Analyze claim scope"

# Pipe from stdin
echo "Explain prior art for solar panel patents" | flowleap chat

# Non-streaming with JSON output (best for agents)
flowleap chat --no-stream --output json "What is patent EP1234567 about?"
```

## Response Format (JSON)

Non-streaming response:
```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "The response text..."
    },
    "finish_reason": "stop"
  }]
}
```

Streaming sends SSE events with `delta.content` fragments.

## Agent Usage

For AI agents, always use `--no-stream --output json` to get a single parseable response:
```bash
flowleap chat --no-stream --output json "Your question here"
```
