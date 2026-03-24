#[cfg(test)]
mod tests {
    use crate::block::render_blocks;
    use crate::converter::{convert, ConvertOptions};

    /// Minimal options: no heading markers, no code-under-para — for testing core structure.
    fn plain() -> ConvertOptions {
        ConvertOptions {
            flat_headings: false,
            heading_markers: false,
            split_paragraphs: false,
            code_under_para: false,
        }
    }

    fn run(input: &str) -> String {
        let blocks = convert(input, &plain());
        render_blocks(&blocks)
    }

    fn run_opts(input: &str, opts: ConvertOptions) -> String {
        let blocks = convert(input, &opts);
        render_blocks(&blocks)
    }

    // -----------------------------------------------------------------------
    // Rule 1: Headings → nested blocks
    // -----------------------------------------------------------------------

    #[test]
    fn heading_nesting() {
        let input = "# H1\n\n## H2\n\n### H3\n";
        let out = run(input);
        assert_eq!(out, "- H1\n  - H2\n    - H3\n");
    }

    #[test]
    fn paragraph_under_heading() {
        let input = "# Title\n\nSome text.\n";
        let out = run(input);
        assert_eq!(out, "- Title\n  - Some text.\n");
    }

    #[test]
    fn sibling_headings() {
        let input = "# A\n\ntext a\n\n# B\n\ntext b\n";
        let out = run(input);
        assert_eq!(out, "- A\n  - text a\n- B\n  - text b\n");
    }

    // -----------------------------------------------------------------------
    // Rule 2: Bare paragraph → block
    // -----------------------------------------------------------------------

    #[test]
    fn bare_paragraph() {
        let input = "Hello world.\n";
        let out = run(input);
        assert_eq!(out, "- Hello world.\n");
    }

    // -----------------------------------------------------------------------
    // Rule 3: Lists keep structure
    // -----------------------------------------------------------------------

    #[test]
    fn list_structure_preserved() {
        let input = "- item1\n  - sub\n- item2\n";
        let out = run(input);
        assert_eq!(out, "- item1\n  - sub\n- item2\n");
    }

    // -----------------------------------------------------------------------
    // Rule 4: Code block as single multiline block
    // -----------------------------------------------------------------------

    #[test]
    fn code_block_nested() {
        let input = "## Section\n\n```python\nprint(\"hi\")\n```\n";
        let out = run(input);
        assert!(out.contains("- Section\n"), "heading should be a block");
        assert!(out.contains("  - ```python\n"), "code fence under heading");
        assert!(out.contains("    print(\"hi\")\n"), "code line as continuation");
        assert!(out.contains("    ```\n"), "closing fence as continuation");
    }

    #[test]
    fn code_under_para_default() {
        // With default options (code_under_para: true), code nests under preceding paragraph.
        let input = "One rule:\n\n```bash\necho hi\n```\n";
        let out = run_opts(input, ConvertOptions::default());
        assert!(out.contains("- One rule:\n"), "paragraph is parent block");
        assert!(out.contains("  - ```bash\n"), "code is child block");
    }

    // -----------------------------------------------------------------------
    // Rule 5: Blockquote
    // -----------------------------------------------------------------------

    #[test]
    fn blockquote_content() {
        let input = "> Some quoted text.\n";
        let out = run(input);
        assert_eq!(out, "- Some quoted text.\n");
    }

    // -----------------------------------------------------------------------
    // Rule 6: Bold / italic preserved
    // -----------------------------------------------------------------------

    #[test]
    fn bold_and_italic_preserved() {
        let input = "**bold** and *italic*.\n";
        let out = run(input);
        assert_eq!(out, "- **bold** and *italic*.\n");
    }

    // -----------------------------------------------------------------------
    // Rule 7: Links preserved
    // -----------------------------------------------------------------------

    #[test]
    fn link_preserved() {
        let input = "[Google](https://google.com)\n";
        let out = run(input);
        assert_eq!(out, "- [Google](https://google.com)\n");
    }

    // -----------------------------------------------------------------------
    // Table → single multiline block
    // -----------------------------------------------------------------------

    #[test]
    fn table_to_list() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let out = run(input);
        assert!(out.contains("- | A | B |"), "header row in first line of block");
        assert!(out.contains("  | --- | --- |"), "separator as continuation line");
        assert!(out.contains("  | 1 | 2 |"), "data row as continuation line");
    }

    // -----------------------------------------------------------------------
    // --heading-markers (default on)
    // -----------------------------------------------------------------------

    #[test]
    fn heading_markers_default() {
        let input = "# H1\n\n## H2\n";
        let out = run_opts(input, ConvertOptions::default());
        assert!(out.contains("- # H1\n"), "H1 should have # prefix");
        assert!(out.contains("  - ## H2\n"), "H2 should have ## prefix");
    }

    // -----------------------------------------------------------------------
    // --flat-headings option
    // -----------------------------------------------------------------------

    #[test]
    fn flat_headings_mode() {
        let input = "# Title\n\nParagraph.\n\n## Sub\n\nContent.\n";
        let out = run_opts(input, ConvertOptions { flat_headings: true, ..plain() });
        assert!(out.contains("- **Title**\n"), "heading should be bolded");
        assert!(out.contains("  - Paragraph.\n"), "paragraph should NOT be bolded");
        assert!(out.contains("  - **Sub**\n"), "sub-heading should be bolded");
    }

    // -----------------------------------------------------------------------
    // --split-paragraphs option
    // -----------------------------------------------------------------------

    #[test]
    fn split_paragraphs_mode() {
        let input = "line one\nline two\nline three\n";
        let out = run_opts(input, ConvertOptions { split_paragraphs: true, ..plain() });
        assert!(out.contains("- line one\n"), "first line");
        assert!(out.contains("- line two\n"), "second line");
        assert!(out.contains("- line three\n"), "third line");
    }
}
