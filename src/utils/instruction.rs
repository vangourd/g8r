use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub token: String,           // "__INSTRUCTION_1__"
    pub instruction_type: String, // "g8r_output", "g8r_secret", etc.
    pub args: Vec<String>,       // Function arguments
    pub target_path: String,     // JSON path where to substitute the value
    pub expected_type: Option<String>, // Optional type constraint
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionContext {
    pub instructions: Vec<Instruction>,
    pub next_token_id: usize,
}

impl InstructionContext {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            next_token_id: 1,
        }
    }

    pub fn add_instruction(
        &mut self, 
        instruction_type: String,
        args: Vec<String>,
        target_path: String,
        expected_type: Option<String>,
    ) -> String {
        let token = format!("__INSTRUCTION_{}__", self.next_token_id);
        self.next_token_id += 1;

        let instruction = Instruction {
            token: token.clone(),
            instruction_type,
            args,
            target_path,
            expected_type,
        };

        self.instructions.push(instruction);
        token
    }

    pub fn has_instructions(&self) -> bool {
        !self.instructions.is_empty()
    }
}

impl Default for InstructionContext {
    fn default() -> Self {
        Self::new()
    }
}