use crate::prelude::*;

/// Returns tokens count in string
pub fn count_tokens(text: &str) -> usize {
    let tokenizer = tiktoken_rs::cl100k_base().expect("Failed to create tokenizer");
    tokenizer.encode_with_special_tokens(text).len()
}

/// Cuts AI context by max tokens len
pub fn cut_context_lines(lines: &[String], limit: usize) -> String {
    let mut sum_count = 0;

    // filter last lines:
    let mut filtered = lines
        .iter()
        .rev()
        .filter(|line| {
            let count = utils::count_tokens(line);
            if sum_count + count <= limit {
                sum_count += count;
                true
            } else {
                false
            }
        })
        .map(Clone::clone)
        .collect::<Vec<_>>();

    filtered.reverse();
    filtered.join("\n")
}
