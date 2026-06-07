use lab_core::io::{load_annotation, load_meta};

fn main() {
    // Load meta.yaml
    println!("Loading meta.yaml...");
    match load_meta("label_root/meta.yaml") {
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
    println!("Loading labels/0001.yaml...");
    match load_annotation("label_root/labels/0001.yaml") {
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
