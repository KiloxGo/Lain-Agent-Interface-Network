use std::fs;

pub struct Config {
    pub system_prompt: String,
}

impl Config {
    pub fn load() -> Self {
        let system_prompt = fs::read_to_string("prompts/lain_system.md")
        .unwrap_or_else(|_| {
            eprintln!("Warning: lain_system not found,who am i");
            "You are Lain.".to_string()
        });
        Self{system_prompt}
    }
}