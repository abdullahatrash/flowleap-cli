---
name: flowleap-ocr
version: 1.0.0
description: "FlowLeap OCR: Extract text from PDFs and images via Mistral."
metadata:
  category: "patent-ai"
  requires:
    bins: ["flowleap"]
  cliHelp: "flowleap ocr --help"
---

# FlowLeap OCR

Prerequisite: Read `flowleap-shared` for authentication and global flags.

## Usage

```bash
flowleap ocr extract <file> [flags]
```

Uploads a PDF or image file to `/v1/ocr` via multipart form. Uses Mistral for text extraction.

## Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--format` | Output format: `text`, `markdown` | `markdown` |

## Examples

```bash
# Extract text from PDF
flowleap ocr extract patent-document.pdf

# Extract as plain text
flowleap ocr extract scan.png --format text

# JSON output for agents
flowleap ocr extract document.pdf --output json
```

## Response Format (JSON)

```json
{
  "text": "Extracted text content from the document..."
}
```

## Supported File Types

- PDF documents
- PNG, JPG, JPEG images
- TIFF images

## Safety

The file must exist locally. The CLI validates the file path before uploading.
