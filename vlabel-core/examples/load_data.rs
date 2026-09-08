use vlabel_core::io::{load_annotation, load_meta};

fn main() {
    // Load meta.json5
    println!("Loading meta.json5...");
    match load_meta("label_root/meta.json5") {
        Ok(meta) => {
            println!("  ID: {}", meta.id);
            println!("  Name: {}", meta.name);
            println!("  Description: {}", meta.description);
            println!("  Categories: {}", meta.categories.len());
            println!("  Property types: {}", meta.property_types.len());
        }
        Err(e) => {
            eprintln!("Failed to load meta: {}", e);
        }
    }

    println!();

    // Load annotation
    println!("Loading vlabels/0001.json5...");
    match load_annotation("label_root/vlabels/0001.json5") {
        Ok(label) => {
            println!("  Version: {}", label.version);
            println!("  User agent: {}", label.user_agent);
            println!("  Objects: {}", label.objects.len());
            println!("  ROI count: {}", label.rois.len());

            for (i, obj) in label.objects.iter().enumerate() {
                println!(
                    "  Object {}: category={}, points={}, properties={}",
                    i,
                    obj.category,
                    obj.polygon.0.len(),
                    obj.properties.len()
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to load annotation: {}", e);
        }
    }
}
