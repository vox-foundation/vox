use tokenizers::Tokenizer;
use vox_tensor::data::{TrainingPair, ChatmlConfig};

fn main() -> anyhow::Result<()> {
    let tokenizer = Tokenizer::from_file("c:\\Users\\Owner\\vox\\mens\\data\\tokenizer.json").unwrap();
    let pair_json = r#"{"category":"function","difficulty":2,"instruction":"Write a Vox function called DatabaseConfig","lane":"vox_codegen","origin":"human","output":"type DatabaseConfig = | DatabaseConfig(url: str, pool_size: int)","prompt":"Write a Vox function called DatabaseConfig","rating":5,"response":"type DatabaseConfig = | DatabaseConfig(url: str, pool_size: int)"}"#;
    let pair: TrainingPair = serde_json::from_str(pair_json)?;
    let system_prompt = "You are a Vox coding expert.";
    let chatml = ChatmlConfig::default();

    let full_text = format!(
        "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{}<|im_end|>",
        system_prompt, pair.prompt.as_ref().unwrap(), pair.response.as_ref().unwrap()
    );
    let prefix_text = format!(
        "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
        system_prompt, pair.prompt.as_ref().unwrap()
    );

    let full_ids = tokenizer.encode(full_text, true).unwrap().get_ids().to_vec();
    let prefix_ids = tokenizer.encode(prefix_text, true).unwrap().get_ids().to_vec();

    println!("Full IDs length: {}", full_ids.len());
    println!("Prefix IDs length: {}", prefix_ids.len());

    let mut matched = 0usize;
    let upper = prefix_ids.len().min(full_ids.len());
    for i in 0..upper {
        if prefix_ids[i] == full_ids[i] {
            matched = i + 1;
        } else {
            break;
        }
    }
    println!("Matched prefix tokens: {}", matched);

    let ce_last_k = 64;
    let last_k_start = full_ids.len().saturating_sub(ce_last_k);
    println!("last_k_start: {}", last_k_start);

    let mut eligible = 0;
    for i in 0..full_ids.len() - 1 {
        let target_idx = i + 1;
        if target_idx >= matched && target_idx >= last_k_start {
            eligible += 1;
        }
    }
    println!("Eligible tokens for CE: {}", eligible);

    Ok(())
}
