use vox_tensor::data::load_all;
use std::path::PathBuf;

fn main() {
    let p = PathBuf::from(r"C:\Users\Owner\vox\target\dogfood\train.jsonl");
    println!("Loading from {:?}", p);
    if !p.exists() {
        println!("File does not exist!");
        return;
    }
    match load_all(&p, 3) {
        Ok(pairs) => {
            println!("Load success: {} pairs", pairs.len());
            if pairs.is_empty() {
                // Try again with rating 0 to see if it's the rating or the parsing
                match load_all(&p, 0) {
                    Ok(all) => println!("Total pairs with rating>=0: {}", all.len()),
                    Err(e) => println!("Error with rating 0: {:?}", e),
                }
            }
        }
        Err(e) => println!("Error loading: {:?}", e),
    }
}
