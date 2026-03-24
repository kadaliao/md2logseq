use clap::Parser;
use std::io::{self, Read, Write};
use std::path::PathBuf;

mod block;
mod converter;
#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
#[command(
    name = "md2logseq",
    about = "Convert Markdown to Logseq block format",
    long_about = "Convert standard Markdown (GFM) to Logseq-compatible block Markdown.\n\
                  \n\
                  Default behaviour (no flags needed):\n\
                  - Headings become nested blocks with # markers (Logseq styled)\n\
                  - Lists nest under the preceding paragraph\n\
                  - Code blocks nest under the preceding paragraph\n\
                  - Tables are preserved as single multiline blocks\n\
                  - Inline markup (bold, italic, links, images) is kept as-is\n\
                  \n\
                  Use --no-heading-markers or --no-code-under-para to disable defaults."
)]
struct Cli {
    /// Input Markdown file (reads from stdin if omitted)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output file (writes to stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// [off by default] Replace heading text with bold text (##-style labels removed)
    #[arg(long)]
    flat_headings: bool,

    /// [on by default] Disable # markers on heading blocks (Logseq styled headings)
    #[arg(long)]
    no_heading_markers: bool,

    /// [off by default] Split multi-line paragraphs: each line becomes a separate sibling block
    #[arg(long)]
    split_paragraphs: bool,

    /// [on by default] Disable nesting code blocks under the preceding paragraph
    #[arg(long)]
    no_code_under_para: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let input = match &cli.input {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    let opts = converter::ConvertOptions {
        flat_headings: cli.flat_headings,
        heading_markers: !cli.no_heading_markers,
        split_paragraphs: cli.split_paragraphs,
        code_under_para: !cli.no_code_under_para,
    };

    let blocks = converter::convert(&input, &opts);
    let output = block::render_blocks(&blocks);

    match &cli.output {
        Some(path) => std::fs::write(path, output)?,
        None => io::stdout().write_all(output.as_bytes())?,
    }

    Ok(())
}
