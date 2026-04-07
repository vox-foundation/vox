fn bad_prompt(secret: &str) -> String {
    let system_prompt = format!("system context includes secret {}", secret);
    system_prompt
}
