use horus::memory::shm_base_dir;

fn main() {
    // Use flat namespace - topics are directly under base dir
    let topics_dir = shm_base_dir().join("topics");
    println!("Checking {}...", topics_dir.display());
    if !topics_dir.exists() {
        println!("Topics directory does not exist!");
        return;
    }

    match std::fs::read_dir(&topics_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("Topic file: {:?}", entry.file_name());
                }
            }
        }
        Err(e) => println!("Error reading topics directory: {}", e),
    }
}
