//semantic analysis errors

#[derive(Clone)]
pub struct SemanticError {
    pub message: String,
    pub start: usize,
    pub end: usize,
    pub src: Option<String>,
}

impl SemanticError {
    pub fn new(message: impl Into<String>, start: usize, end: usize, src: Option<String>) -> Self {
        Self {
            message: message.into(),
            start,
            end,
            src,
        }
    }
    pub fn eof(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            start: 0,
            end: 0,
            src: None,
        }
    }
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.message)?;
        if let Some(src) = &self.src {
            let code_line = src.lines().nth(0).unwrap_or("");
            writeln!(f, "  {}", code_line)?;
            writeln!(
                f,
                "  {}{}",
                " ".repeat(self.start),
                "─".repeat(self.end.saturating_sub(self.start).max(1))
            )?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for SemanticError {}
