use crate::rect::Rect;
use std::path::Path;

pub fn process_pdf(input_path: &Path, output_path: &Path, bleed_pts: f64) -> lopdf::Result<()> {
    let mut document = lopdf::Document::load(input_path)?;
    let pages: Vec<lopdf::ObjectId> = document.get_pages().into_values().collect();

    for page_id in &pages {
        let trim = match get_trim_box(&document, *page_id) {
            Some(r) => r,
            None => continue,
        };
        
        let target = trim.expand(bleed_pts);
        let arr = target.to_pdf_array();
        let pdf_arr = lopdf::Object::Array(
            arr.iter()
                .map(|&v| lopdf::Object::Real(v as f32))
                .collect(),
        );
        
        let page = document.get_dictionary_mut(*page_id)?;
        page.set(b"MediaBox".to_vec(), pdf_arr.clone());
        page.set(b"CropBox".to_vec(), pdf_arr.clone());
    }

    document.save(output_path)?;
    Ok(())
}

/// Converts a PDF object to a floating-point number.
///
/// This function extracts numeric values from PDF objects and converts them to `f64`.
/// It supports both integer and real number objects from the lopdf library.
///
/// # Arguments
///
/// * `object` - A reference to a `lopdf::Object` that should contain a numeric value
///
/// # Returns
///
/// * `f64` - The numeric value extracted from the PDF object
///
/// # Panics
///
/// This function will panic if the provided object is not a numeric type
/// (neither `lopdf::Object::Integer` nor `lopdf::Object::Real`).
///
/// # Examples
///
/// ```rust,ignore
/// use lopdf::Object;
///
/// let int_obj = Object::Integer(42);
/// let real_obj = Object::Real(3.14);
///
/// assert_eq!(object_to_f64(&int_obj), 42.0);
/// assert_eq!(object_to_f64(&real_obj), 3.14 as f64);
/// ```
fn object_to_f64(object: &lopdf::Object) -> f64 {
    match object {
        lopdf::Object::Integer(i) => *i as f64,
        lopdf::Object::Real(r) => *r as f64,
        _ => panic!("expected numeric Object, got {:?}", object),
    }
}
fn get_trim_box(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Option<Rect> {
    let page = doc.get_dictionary(page_id).ok()?;
    let box_obj = page
        .get(b"TrimBox")
        .or_else(|_| page.get(b"MediaBox"))
        .ok()?;
    let arr = box_obj.as_array().ok()?;
    if arr.len() < 4 {
        return None;
    }
    let values: Vec<f64> = arr.iter().map(object_to_f64).collect();
    Some(Rect::from_corners(
        values[0], values[1], values[2], values[3],
    ))
}
