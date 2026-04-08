# PDF Resize To Bleed Or Trim

Resize PDF pages to match their **TrimBox** or expand them by a user-defined **bleed size**.

## Constraints

| Rule                   | Detail                                                                                                                       |
|------------------------|------------------------------------------------------------------------------------------------------------------------------|
| **Resize Only**        | MediaBox and CropBox dimensions are the only things that change.                                                             |
| **Defined Bleed**      | User defines a bleed size in thousandths of an inch (e.g. `125` = ⅛") which expands the TrimBox outward. TrimBox is preserved. |
| **No Content Changes** | Page content streams are never decoded, filtered, or re-encoded.                                                             |

> **Note on units:** The current release accepts bleed values in thousandths of
> an inch (1/1000"). Additional unit options may be added in a future version.

## Status

**Beta.** Core algorithm validated against production-level pre-press PDFs. See [Known Limitations](#known-limitations) for current tradeoffs.

---

## Usage

### Single-file mode

```bash
prsz [-b <thousandths>] [-t] <input.pdf> [output.pdf]
```

| Flag / Argument | Description                                                                                              |
|-----------------|----------------------------------------------------------------------------------------------------------|
| `<input.pdf>`   | Path to the source PDF.                                                                                  |
| `[output.pdf]`  | Output path (default: `<input>-resized.pdf` beside the input). A directory is also accepted.             |
| `-b <value>`    | Uniform bleed size in thousandths of an inch to add around the TrimBox (e.g. `-b 125` = ⅛" per side).   |
| `-t`            | Crop MediaBox / CropBox to exactly match the TrimBox (remove all bleed / mark space).                   |

```bash
# Add 1/8 inch bleed around TrimBox
prsz -b 125 artwork.pdf

# Crop page to TrimBox (no bleed)
prsz -t artwork.pdf output.pdf
```

### Batch mode

```bash
prsz [-b <thousandths>] [-t] [input-1.pdf, input-2.pdf, ...] [output/dir]
```

Pass a comma-separated list of input paths enclosed in square brackets, with
an optional output directory as the final argument.

| Argument           | Description                                                                                                   |
|--------------------|---------------------------------------------------------------------------------------------------------------|
| `[input-1.pdf, …]` | One or more PDF paths separated by commas, wrapped in `[` and `]`. Spaces around commas and paths are ignored. |
| `[output/dir]`     | Directory where every resized file is written (optional). Defaults to the same directory as each input file.  |

Each file is written as `<stem>-resized.pdf` inside the output directory. If
any input file fails, processing continues and the exit code is 1 at the end.

```bash
# Batch — add 1/8" bleed, all output to a specific directory
prsz -b 125 [art-1.pdf, art-2.pdf, art-3.pdf] output/resized/

# Batch — each resized file beside its input
prsz -b 125 [art-1.pdf, art-2.pdf, art-3.pdf]
```

> **zsh / bash note:** Square brackets are reserved glob syntax in most shells.
> Escape them with backslashes on the command line:
>
> ```bash
> prsz -b 125 \[art-1.pdf, art-2.pdf, art-3.pdf\] output/resized/
> ```
>
> Alternatively, single-quote the entire bracket block:
>
> ```bash
> prsz -b 125 '[art-1.pdf, art-2.pdf, art-3.pdf]' output/resized/
> ```

### Install (global binary)

```bash
cargo install --path .
```

This places the binary at `~/.cargo/bin/prsz`.

### Build

```bash
cargo build --release
```

### Run (without installing)

```bash
cargo run --release -- -b 125 <input.pdf>
```

### Test

```bash
cargo test -- --nocapture
```

---

## Dependencies

- **Language:** Rust (2024 edition)
- **Crate:** [`lopdf 0.40.0`](https://crates.io/crates/lopdf)
  ([source](https://github.com/J-F-Liu/lopdf)) -- low-level PDF manipulation
- No paid services or external tooling required at runtime.

---

## How It Works

### High-level pipeline

1. **Load** the PDF with
   [`lopdf::Document::load`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.load).
2. **Enumerate pages** via
   [`Document::get_pages`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.get_pages),
   which returns a `BTreeMap<u32, ObjectId>` mapping page numbers to object IDs.
3. **Read the TrimBox** from the page dictionary
   ([`Document::get_dictionary`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.get_dictionary)
   and [`Dictionary::get`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Dictionary.html#method.get)).
   The TrimBox is a four-element array `[x0, y0, x1, y1]` stored as
   [`Object::Integer`](https://docs.rs/lopdf/0.40.0/lopdf/enum.Object.html#variant.Integer)
   or [`Object::Real`](https://docs.rs/lopdf/0.40.0/lopdf/enum.Object.html#variant.Real) values.
   See PDF Reference 1.7, Section 14.11.2 -- Page Boundaries.
4. **Compute the target box:**
    - **`-b <n>`**: convert `n` from thousandths of an inch to points
      (`n / 1000 × 72`), then expand the TrimBox outward by that amount on
      each side → `[x0 - pts, y0 - pts, x1 + pts, y1 + pts]`.
    - **`-t`**: use the TrimBox as-is (equivalent to `-b 0`).
5. **Write the new MediaBox and CropBox** into the page dictionary using
   [`Dictionary::set`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Dictionary.html#method.set).
   The TrimBox entry is left untouched so downstream workflows still know the
   intended finished size.
6. **Save** the modified PDF with
   [`Document::save`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.save).

> **Note:** The content stream is never touched. All existing artwork, images,
> and text remain exactly as they are. Only page-level box metadata changes.

### How resizing interacts with existing content

Because the MediaBox and CropBox define the **visible area** of the page, shrinking
them (e.g., `-t`) hides content that lies outside the new boundaries without
deleting it. Expanding them (e.g., `-b 125`) reveals content that was previously
clipped. No drawing instructions are added, removed, or modified.

---

## PDF Background

This section provides context for contributors who are not familiar with the
PDF specification internals that this project relies on.

### Page boxes

A PDF page can define several rectangles (in points, origin at bottom-left).
These are specified in PDF Reference 1.7, Section 14.11.2 (Page Boundaries):

| Box          | Meaning                                                                             |
|--------------|-------------------------------------------------------------------------------------|
| **MediaBox** | Full physical page, including all bleed/mark space -- **resized by this tool**.     |
| TrimBox      | The finished page boundary after cutting -- **used as the reference for resizing**. |
| BleedBox     | Bleed zone extending slightly beyond the TrimBox.                                   |
| **CropBox**  | Visible page area (often same as MediaBox) -- **resized by this tool**.             |

In `lopdf`, page boxes are read from the page
[`Dictionary`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Dictionary.html) via
[`Dictionary::get`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Dictionary.html#method.get),
which returns an [`Object`](https://docs.rs/lopdf/0.40.0/lopdf/enum.Object.html).
Box values are `Object::Array` containing `Object::Integer` or `Object::Real`
elements.

---

## Project Structure

```
Cargo.toml
README.md
src/
    lib.rs          -- crate root: module declarations and public re-exports
    main.rs         -- binary entry point (CLI argument parsing)
    rect.rs         -- Rect struct (bounding box math, expand/contract helpers)
    matrix.rs       -- Matrix struct (2D affine transforms -- retained for future use)
    filter.rs       -- utility functions (object_to_f64 conversion)
    process.rs      -- top-level pipeline (load, resize boxes, save)
    tests.rs        -- unit and integration tests (compiled only in test builds)
test/
    test_assets/    -- PDF fixtures used by integration tests
    test_result/    -- output directory for test runs
```

### Key types and functions

| Item            | Location     | Purpose                                                                 |
|-----------------|--------------|-------------------------------------------------------------------------|
| `Rect`          | `rect.rs`    | Axis-aligned rectangle with `expand(bleed)` for computing new boxes.    |
| `Matrix`        | `matrix.rs`  | 2D affine matrix (retained for potential future coordinate transforms). |
| `process_pdf`   | `process.rs` | End-to-end pipeline: load PDF, resize page boxes, save.                 |
| `object_to_f64` | `filter.rs`  | Converts `lopdf::Object` (Integer or Real) to `f64`.                    |
| `get_trim_box`  | `process.rs` | Reads the TrimBox (or MediaBox fallback) from a page dictionary.        |

### lopdf API surface used

| lopdf item                                                                                                  | Kind       | Used in      | Purpose                                                             |
|-------------------------------------------------------------------------------------------------------------|------------|--------------|---------------------------------------------------------------------|
| [`Document`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html)                                       | struct     | `process.rs` | Top-level PDF document handle.                                      |
| [`Document::load`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.load)                     | method     | `process.rs` | Load a PDF from a file path.                                        |
| [`Document::save`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.save)                     | method     | `process.rs` | Write the modified PDF to disk.                                     |
| [`Document::get_pages`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.get_pages)           | method     | `process.rs` | Get the page-number-to-ObjectId map.                                |
| [`Document::get_dictionary`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Document.html#method.get_dictionary) | method     | `process.rs` | Read a page as a `Dictionary`.                                      |
| [`Dictionary::get`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Dictionary.html#method.get)                   | method     | `process.rs` | Look up a key (e.g. `TrimBox`) in a page dictionary.                |
| [`Dictionary::set`](https://docs.rs/lopdf/0.40.0/lopdf/struct.Dictionary.html#method.set)                   | method     | `process.rs` | Write a new value (e.g. resized `MediaBox`) into a page dictionary. |
| [`Object`](https://docs.rs/lopdf/0.40.0/lopdf/enum.Object.html)                                             | enum       | `process.rs` | PDF object (Integer, Real, Array, etc.).                            |
| [`ObjectId`](https://docs.rs/lopdf/0.40.0/lopdf/type.ObjectId.html)                                         | type alias | `process.rs` | `(u32, u16)` -- object number and generation.                       |

---

## PDF Inspection Tools

These tools are useful for debugging and verifying output:

```bash
# Decompress a PDF into human-readable form
qpdf --qdf --object-streams=disable input.pdf readable.pdf

# Show page boxes (MediaBox, TrimBox, BleedBox, etc.)
pdfinfo input.pdf

# List all embedded images with metadata
pdfimages -list input.pdf

# Structural integrity check
qpdf --check input.pdf
```

Install: `brew install qpdf poppler` (macOS) or `apt install qpdf poppler-utils` (Debian/Ubuntu).

---

## Known Limitations

### Content outside TrimBox is hidden, not deleted

When `-t` is used, content outside the new CropBox/MediaBox still exists in
the PDF file -- it is simply not rendered by viewers. To actually remove that
content, use a content-stream filtering tool such as
[remove-trim-marks-pdf](https://github.com/...) (the sibling project this was
forked from).

### TrimBox is required

If a page has no TrimBox, the tool falls back to the MediaBox. If neither is
present the page is skipped with a warning.

---

## Contributing

Contributions are welcome. When making changes, keep the following in mind:

- Run the full test suite (`cargo test`) before submitting a pull request.
- The TrimBox must never be modified -- it is the source-of-truth reference box.
- Only MediaBox and CropBox should be written.

---

## References

- [PDF Reference 1.7](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/pdfreference1.7old.pdf)
  -- Section 14.11.2 (Page Boundaries)
- [lopdf documentation](https://docs.rs/lopdf/0.40.0/lopdf/)
