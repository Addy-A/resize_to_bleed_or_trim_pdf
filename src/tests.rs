use crate::rect::Rect;
use crate::process::process_pdf;

/*
    To run these tests, use:
        cargo test -- --nocapture
    The --nocapture flag allows us to see printed output during tests.

    To specifically run just one test, use:
        cargo test test_name -- --nocapture
*/

// Shared helper -- loads a PDF and returns the document.
// Paths are relative to the project root via CARGO_MANIFEST_DIR.
const SOURCE_PDF_REL: &str = "test/test_assets/pdf_test_data_print_v2.pdf";

fn test_asset(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn load_document(path: &std::path::Path) -> lopdf::Document {
    let file = std::fs::File::open(path).expect("test PDF not found");
    lopdf::Document::load_from(file).expect("failed to parse PDF")
}

// ── Rect unit tests ─────────────────────────────────────────────────────

#[test]
fn test_from_corners_normalises_order() {
    // Passing corners in reversed order should still produce the same rect
    let a = Rect::from_corners(30.0, 30.0, 642.0, 822.0);
    let b = Rect::from_corners(642.0, 822.0, 30.0, 30.0);
    assert_eq!(a.x, b.x);
    assert_eq!(a.y, b.y);
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);
}

#[test]
fn test_expand_zero_is_identity() {
    let trim = Rect::from_corners(30.0, 30.0, 642.0, 822.0);
    let same = trim.expand(0.0);
    assert_eq!(same.x, trim.x);
    assert_eq!(same.y, trim.y);
    assert_eq!(same.width, trim.width);
    assert_eq!(same.height, trim.height);
}

#[test]
fn test_expand_positive_bleed() {
    // 125 thousandths of an inch = 0.125" = 9.0 points (0.125 * 72)
    let bleed_pts = 9.0;
    let trim = Rect::from_corners(30.0, 30.0, 642.0, 822.0);
    let expanded = trim.expand(bleed_pts);

    assert_eq!(expanded.x, 21.0);           // 30 - 9
    assert_eq!(expanded.y, 21.0);           // 30 - 9
    assert!((expanded.right() - 651.0).abs() < 0.001);  // 642 + 9
    assert!((expanded.top() - 831.0).abs() < 0.001);    // 822 + 9
    assert!((expanded.width - 630.0).abs() < 0.001);    // 612 + 18
    assert!((expanded.height - 810.0).abs() < 0.001);   // 792 + 18
}

#[test]
fn test_to_pdf_array() {
    let r = Rect::from_corners(21.0, 21.0, 651.0, 831.0);
    let arr = r.to_pdf_array();
    assert_eq!(arr, [21.0, 21.0, 651.0, 831.0]);
}

#[test]
fn test_to_pdf_array_matches_expand() {
    let trim = Rect::from_corners(30.0, 30.0, 642.0, 822.0);
    let expanded = trim.expand(9.0);
    let arr = expanded.to_pdf_array();
    assert_eq!(arr[0], expanded.x);
    assert_eq!(arr[1], expanded.y);
    assert!((arr[2] - expanded.right()).abs() < 0.001);
    assert!((arr[3] - expanded.top()).abs() < 0.001);
}

// ── Unit conversion test ────────────────────────────────────────────────

#[test]
fn test_thousandths_to_points_conversion() {
    // The conversion formula: thousandths / 1000.0 * 72.0
    let convert = |thousandths: u32| thousandths as f64 / 1000.0 * 72.0;

    assert!((convert(125) - 9.0).abs() < 0.001);       // 1/8"
    assert!((convert(250) - 18.0).abs() < 0.001);      // 1/4"
    assert!((convert(500) - 36.0).abs() < 0.001);      // 1/2"
    assert!((convert(1000) - 72.0).abs() < 0.001);     // 1"
}

// ── Integration: round-trip PDF processing ──────────────────────────────

#[test]
fn test_process_pdf_trim_to_trimbox() {
    let source = test_asset(SOURCE_PDF_REL);
    if !source.exists() {
        eprintln!("Skipping integration test: {} not found", SOURCE_PDF_REL);
        return;
    }

    let output_dir = test_asset("test/test_result");
    std::fs::create_dir_all(&output_dir).expect("could not create test_result dir");
    let output = output_dir.join("trimmed_to_trimbox.pdf");

    // Process with bleed = 0 (equivalent to -t)
    process_pdf(&source, &output, 0.0).expect("process_pdf failed");

    // Reload and verify MediaBox/CropBox match the TrimBox
    let doc = load_document(&output);
    let pages = doc.get_pages();
    for (_, page_id) in &pages {
        let page = doc.get_dictionary(*page_id).expect("page dict");

        let trim_arr = page.get(b"TrimBox")
            .or_else(|_| page.get(b"MediaBox"))
            .expect("no TrimBox or MediaBox");
        let trim_vals: Vec<f64> = trim_arr.as_array().unwrap()
            .iter()
            .map(|o| match o {
                lopdf::Object::Integer(i) => *i as f64,
                lopdf::Object::Real(r) => *r as f64,
                _ => panic!("non-numeric box value"),
            })
            .collect();

        let media_arr = page.get(b"MediaBox").expect("no MediaBox after processing");
        let media_vals: Vec<f64> = media_arr.as_array().unwrap()
            .iter()
            .map(|o| match o {
                lopdf::Object::Integer(i) => *i as f64,
                lopdf::Object::Real(r) => *r as f64,
                _ => panic!("non-numeric box value"),
            })
            .collect();

        for i in 0..4 {
            assert!(
                (media_vals[i] - trim_vals[i]).abs() < 0.5,
                "MediaBox[{}] = {} but TrimBox[{}] = {}",
                i, media_vals[i], i, trim_vals[i]
            );
        }
    }
    println!("test_process_pdf_trim_to_trimbox: PASSED — output at {}", output.display());
}

#[test]
fn test_process_pdf_expand_bleed() {
    let source = test_asset(SOURCE_PDF_REL);
    if !source.exists() {
        eprintln!("Skipping integration test: {} not found", SOURCE_PDF_REL);
        return;
    }

    let output_dir = test_asset("test/test_result");
    std::fs::create_dir_all(&output_dir).expect("could not create test_result dir");
    let output = output_dir.join("expanded_bleed.pdf");

    // 125 thousandths = 1/8" = 9 points
    let bleed_pts = 125.0_f64 / 1000.0 * 72.0;
    process_pdf(&source, &output, bleed_pts).expect("process_pdf failed");

    // Reload and verify MediaBox is TrimBox expanded by 9pt per side
    let doc = load_document(&output);
    let original = load_document(&source);
    let pages = doc.get_pages();
    let orig_pages = original.get_pages();

    for (page_num, page_id) in &pages {
        let orig_page_id = orig_pages.get(page_num).expect("page missing in original");
        let orig_page = original.get_dictionary(*orig_page_id).expect("orig page dict");

        let trim_arr = orig_page.get(b"TrimBox")
            .or_else(|_| orig_page.get(b"MediaBox"))
            .expect("no TrimBox in original");
        let trim_vals: Vec<f64> = trim_arr.as_array().unwrap()
            .iter()
            .map(|o| match o {
                lopdf::Object::Integer(i) => *i as f64,
                lopdf::Object::Real(r) => *r as f64,
                _ => panic!("non-numeric"),
            })
            .collect();
        let trim_rect = Rect::from_corners(trim_vals[0], trim_vals[1], trim_vals[2], trim_vals[3]);
        let expected = trim_rect.expand(bleed_pts);
        let expected_arr = expected.to_pdf_array();

        let page = doc.get_dictionary(*page_id).expect("page dict");
        let media_arr = page.get(b"MediaBox").expect("no MediaBox");
        let media_vals: Vec<f64> = media_arr.as_array().unwrap()
            .iter()
            .map(|o| match o {
                lopdf::Object::Integer(i) => *i as f64,
                lopdf::Object::Real(r) => *r as f64,
                _ => panic!("non-numeric"),
            })
            .collect();

        for i in 0..4 {
            assert!(
                (media_vals[i] - expected_arr[i]).abs() < 0.5,
                "Page {} MediaBox[{}] = {} but expected {}",
                page_num, i, media_vals[i], expected_arr[i]
            );
        }
    }
    println!("test_process_pdf_expand_bleed: PASSED — output at {}", output.display());
}

#[test]
fn test_trimbox_preserved_after_processing() {
    let source = test_asset(SOURCE_PDF_REL);
    if !source.exists() {
        eprintln!("Skipping integration test: {} not found", SOURCE_PDF_REL);
        return;
    }

    let output_dir = test_asset("test/test_result");
    std::fs::create_dir_all(&output_dir).expect("could not create test_result dir");
    let output = output_dir.join("trimbox_preserved.pdf");

    process_pdf(&source, &output, 9.0).expect("process_pdf failed");

    let original = load_document(&source);
    let doc = load_document(&output);
    let pages = doc.get_pages();
    let orig_pages = original.get_pages();

    for (page_num, page_id) in &pages {
        let orig_page_id = orig_pages.get(page_num).expect("page missing");
        let orig_page = original.get_dictionary(*orig_page_id).expect("orig dict");
        let page = doc.get_dictionary(*page_id).expect("page dict");

        // TrimBox must exist and be unchanged
        if let Ok(orig_trim) = orig_page.get(b"TrimBox") {
            let new_trim = page.get(b"TrimBox").expect("TrimBox was removed!");
            let orig_vals: Vec<f64> = orig_trim.as_array().unwrap()
                .iter()
                .map(|o| match o {
                    lopdf::Object::Integer(i) => *i as f64,
                    lopdf::Object::Real(r) => *r as f64,
                    _ => panic!("non-numeric"),
                })
                .collect();
            let new_vals: Vec<f64> = new_trim.as_array().unwrap()
                .iter()
                .map(|o| match o {
                    lopdf::Object::Integer(i) => *i as f64,
                    lopdf::Object::Real(r) => *r as f64,
                    _ => panic!("non-numeric"),
                })
                .collect();
            assert_eq!(orig_vals, new_vals,
                "Page {} TrimBox was modified!", page_num);
        }
    }
    println!("test_trimbox_preserved_after_processing: PASSED");
}
