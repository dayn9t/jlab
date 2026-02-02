use lab_utils::Project;

fn main() {
    println!("Testing labels directory naming...\n");

    // Test 1: New naming (labels/)
    println!("Test 1: New naming (labels/)");
    match Project::open("/tmp/test_jlab_project") {
        Ok(project) => {
            let labels_dir = project.labels_dir();
            println!("  ✓ Project opened successfully");
            println!("  Labels directory: {:?}", labels_dir);
            println!("  Directory exists: {}", labels_dir.exists());
        }
        Err(e) => {
            println!("  ✗ Failed to open project: {}", e);
        }
    }

    println!();

    // Test 2: Main project (label_root with labels/)
    println!("Test 2: Main project (label_root with labels/)");
    match Project::open("label_root") {
        Ok(project) => {
            let labels_dir = project.labels_dir();
            println!("  ✓ Project opened successfully");
            println!("  Labels directory: {:?}", labels_dir);
            println!("  Directory exists: {}", labels_dir.exists());

            // List annotation files
            if let Ok(entries) = std::fs::read_dir(&labels_dir) {
                let count = entries.count();
                println!("  Annotation files found: {}", count);
            }
        }
        Err(e) => {
            println!("  ✗ Failed to open project: {}", e);
        }
    }

    println!("\n✅ All tests completed!");
}
