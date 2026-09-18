## Handy + Google Gemini 3.5 Transcribe 🎙️

Soft fork of [Handy](https://github.com/cjpais/handy) that adds support for [Gemini Transcribe](https://blog.google/innovation-and-ai/models-and-research/gemini-models/gemini-3-5-transcribe/) as a model provider. Nothing more, nothing less. 

<img width="577" height="97" alt="image" src="https://github.com/user-attachments/assets/e9947ada-01de-44fb-b7a6-799b18401f48" />

<img width="1027" height="664" alt="image" src="https://github.com/user-attachments/assets/71f82abc-90f8-4a5e-a68a-5cff310e30a4" />


## Quick Start

**Point your AI agent to this page and ask it to set it up for you!**

### Pre-requisites:

```
Rust (via rustup.rs) — provides cargo & rustc.
Bun (via bun.sh or brew install oven-sh/bun/bun) — frontend package manager.
C / C++ Build Tools: Xcode Command Line Tools (xcode-select --install).
```

### 1. Clone & Run

```bash
# Clone the repository
git clone https://github.com/epicwhale/handy-gemini.git
cd handy-gemini

# Download Silero VAD model (required)
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx

# Install dependencies and start
bun install
bun run tauri dev
```

### 2. Configure

1. Get a Gemini API key from [Google AI Studio](https://aistudio.google.com/).
2. In Handy, open **Settings → Models**.
3. Paste your key in the **Gemini API Key** field.
4. Select **Gemini 3.5 Transcribe (Cloud)** as your active model.

enjoy! - [dayson@](https://x.com/dayson)

ps. [read the pr/commit](https://github.com/cjpais/Handy/compare/main...epicwhale:handy-gemini:main)
