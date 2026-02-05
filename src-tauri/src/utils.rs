// Utility functions for the AI Story Writer

/// Safely truncate a string to a maximum number of characters
pub fn truncate_string(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}