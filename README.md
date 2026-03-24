# md2logseq

A CLI tool that converts standard Markdown (GFM) to [Logseq](https://logseq.com/)-compatible block Markdown.

## What it does

Logseq uses an outliner format where every line is a `- ` prefixed block. This tool converts regular Markdown into that format:

- Headings become nested blocks (H2 inside H1, H3 inside H2, etc.)
- Paragraphs and list items become blocks
- Code blocks are preserved as multiline blocks, optionally nested under the preceding paragraph
- Tables are preserved as single multiline blocks
- Inline markup (bold, italic, links, images, strikethrough) is kept as-is

### Example

Input:

````markdown
# Getting Started

Install the tool:

```bash
cargo install md2logseq
```

## Usage

Run with `--help` for options.
````

Output (Logseq block format):

````
- # Getting Started
  - Install the tool:
    - ```bash
      cargo install md2logseq
      ```
  - ## Usage
    - Run with `--help` for options.
````

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# binary at: target/release/md2logseq
```

## Usage

```
md2logseq [OPTIONS]

Options:
  -i, --input <FILE>       Input Markdown file (reads from stdin if omitted)
  -o, --output <FILE>      Output file (writes to stdout if omitted)
      --flat-headings      Replace heading text with bold text (## markers removed)
      --no-heading-markers Disable # markers on heading blocks
      --split-paragraphs   Split multi-line paragraphs: each line becomes a separate block
      --no-code-under-para Disable nesting code blocks under the preceding paragraph
  -h, --help               Print help
```

### Pipe usage

```bash
cat notes.md | md2logseq > notes-logseq.md
md2logseq -i notes.md -o notes-logseq.md
```

## Options explained

| Option | Default | Description |
|---|---|---|
| `--flat-headings` | off | Heading text is bolded (`**Title**`) instead of using `# Title` syntax |
| `--no-heading-markers` | on | Disables the `#`/`##` prefix on heading blocks |
| `--split-paragraphs` | off | Soft line breaks within a paragraph create separate sibling blocks |
| `--no-code-under-para` | on | Disables nesting code blocks under the preceding paragraph |

## Development

```bash
cargo test
cargo run -- -i input.md
```

## License

MIT
