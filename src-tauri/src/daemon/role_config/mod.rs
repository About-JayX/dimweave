mod claude_prompt;
pub mod roles;
pub use claude_prompt::claude_system_prompt;
pub use roles::{get_role, output_schema};
