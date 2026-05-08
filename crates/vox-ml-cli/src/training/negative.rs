//! Negative (broken) example generation for training.

/// Mutation strategies for generating negative (broken code) examples.
/// Each returns a (broken_code, error_description) pair.
pub fn generate_negative_examples(code: &str) -> Vec<(String, String)> {
    let mut negatives = Vec::new();

    // Strategy 1: Remove a closing bracket/paren
    if let Some(idx) = code.rfind('}') {
        let mut broken = code.to_string();
        broken.remove(idx);
        negatives.push((broken, "Missing closing brace".to_string()));
    } else if let Some(idx) = code.rfind(')') {
        let mut broken = code.to_string();
        broken.remove(idx);
        negatives.push((broken, "Missing closing parenthesis".to_string()));
    }

    // Strategy 2: Swap 'fn' with 'fun' (invalid keyword)
    if code.contains("fn ") {
        let broken = code.replacen("fn ", "fun ", 1);
        negatives.push((broken, "Invalid keyword 'fun' (should be 'fn')".to_string()));
    }

    // Strategy 3: Remove type annotation
    for line in code.lines() {
        let trimmed = line.trim();
        if let Some(colon_idx) = trimmed.find(") to ") {
            let broken = code.replacen(&trimmed[colon_idx..], "):", 1);
            // Make sure we actually changed something
            if broken != code {
                negatives.push((broken, "Missing return type annotation".to_string()));
                break;
            }
        }
    }

    // Strategy 4: Mangle an identifier
    if code.contains("let ") {
        let broken = code.replacen("let ", "lett ", 1);
        negatives.push((
            broken,
            "Misspelled keyword 'lett' (should be 'let')".to_string(),
        ));
    }

    negatives
}
