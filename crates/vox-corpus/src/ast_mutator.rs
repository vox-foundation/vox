use vox_compiler::ast::decl::Module;

#[derive(Debug)]
pub struct Mutation {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

pub fn generate_mutations(source: &str, _module: &Module) -> Vec<Mutation> {
    // Collect all identifier spans from the source string using simple regex matching 
    // against known AST locations. For full fidelity, we should walk the AST, 
    // but a fast pass with regex verified by the compiler is safer and simpler for SFT.
    let mut mutations = Vec::new();
    let re = regex::Regex::new(r"\b([a-z][a-zA-Z0-9]*[A-Z][a-zA-Z0-9]*)\b").unwrap();
    
    for cap in re.captures_iter(source) {
        if let Some(m) = cap.get(1) {
            let original = m.as_str();
            let snake = to_snake_case(original);
            mutations.push(Mutation {
                start: m.start(),
                end: m.end(),
                replacement: snake,
            });
        }
    }
    
    mutations
}

pub fn apply_mutations(source: &str, mut mutations: Vec<Mutation>) -> String {
    mutations.sort_by_key(|m| m.start);
    let mut result = String::with_capacity(source.len());
    let mut last_end = 0;
    
    for m in mutations {
        if m.start >= last_end {
            result.push_str(&source[last_end..m.start]);
            result.push_str(&m.replacement);
            last_end = m.end;
        }
    }
    result.push_str(&source[last_end..]);
    result
}

fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            for lc in c.to_lowercase() {
                snake.push(lc);
            }
        } else {
            snake.push(c);
        }
    }
    snake
}
