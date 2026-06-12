use super::*;


// ---------------------------------------------------------------------------
// Compile error types
// ---------------------------------------------------------------------------

/// A suggestion attached to a compile error, providing a helpful hint
/// or a possible replacement.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub message: String,
    pub replacement: Option<String>,
}

impl fmt::Display for Suggestion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.replacement {
            Some(rep) => write!(f, "{} (did you mean '{}'?)", self.message, rep),
            None => write!(f, "{}", self.message),
        }
    }
}

/// A semantic error produced by the analyzer.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    /// The primary error message.
    pub message: String,
    /// Optional suggestions for fixing the error.
    pub suggestions: Vec<Suggestion>,
}

impl CompileError {
    /// Create a new error with just a message.
    pub fn new(msg: impl Into<String>) -> Self {
        CompileError {
            message: msg.into(),
            suggestions: Vec::new(),
        }
    }

    /// Create a new error with a message and a single suggestion.
    pub fn with_suggestion(msg: impl Into<String>, suggestion: Suggestion) -> Self {
        CompileError {
            message: msg.into(),
            suggestions: vec![suggestion],
        }
    }

    /// Add a suggestion to this error.
    pub fn suggest(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Check if the error message contains the given substring.
    /// This searches the primary message and all suggestion messages.
    pub fn contains(&self, pattern: &str) -> bool {
        if self.message.contains(pattern) {
            return true;
        }
        for s in &self.suggestions {
            if s.message.contains(pattern) || s.replacement.as_ref().map_or(false, |r| r.contains(pattern)) {
                return true;
            }
        }
        false
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        for s in &self.suggestions {
            write!(f, "\n  help: {}", s)?;
        }
        Ok(())
    }
}

impl From<String> for CompileError {
    fn from(msg: String) -> Self {
        CompileError::new(msg)
    }
}

impl From<&str> for CompileError {
    fn from(msg: &str) -> Self {
        CompileError::new(msg)
    }
}

// ---------------------------------------------------------------------------
// Levenshtein distance for name suggestions
// ---------------------------------------------------------------------------

/// Compute the Levenshtein edit distance between two strings.
pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr: Vec<usize> = vec![0; b_len + 1];

    for (i, ac) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, bc) in b.chars().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1)   // deletion
                .min(curr[j] + 1)               // insertion
                .min(prev[j] + cost);           // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

/// Find symbol names in scope that are similar to the given name.
/// Returns names whose Levenshtein distance is at most `max_distance`.
pub(super) fn find_similar_names(name: &str, scope: &Rc<RefCell<Scope>>, max_distance: usize) -> Vec<String> {
    let all_names = scope.borrow().all_names();
    let mut similar: Vec<(usize, String)> = all_names
        .into_iter()
        .filter_map(|n| {
            let dist = levenshtein(name, &n);
            if dist <= max_distance && dist > 0 {
                Some((dist, n))
            } else {
                None
            }
        })
        .collect();
    similar.sort_by_key(|(d, _)| *d);
    similar.into_iter().map(|(_, n)| n).collect()
}
