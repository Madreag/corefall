//! M8 — Action prompt HUD widget (contextual: "PRESS E TO PICK UP" etc.).

use bevy::prelude::*;

/// Action prompt widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct ActionPromptState {
    /// Active prompt label (empty = no prompt).
    pub prompt: Option<String>,
}

impl ActionPromptState {
    /// Show a prompt (replaces any existing one).
    pub fn show(&mut self, prompt: impl Into<String>) {
        self.prompt = Some(prompt.into());
    }

    /// Clear the prompt.
    pub fn clear(&mut self) {
        self.prompt = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_replaces_prompt() {
        let mut s = ActionPromptState::default();
        s.show("PRESS E TO PICK UP");
        assert_eq!(s.prompt.as_deref(), Some("PRESS E TO PICK UP"));
        s.show("PRESS X TO CROUCH");
        assert_eq!(s.prompt.as_deref(), Some("PRESS X TO CROUCH"));
    }

    #[test]
    fn clear_removes_prompt() {
        let mut s = ActionPromptState::default();
        s.show("PRESS E TO PICK UP");
        s.clear();
        assert!(s.prompt.is_none());
    }
}
