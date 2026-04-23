use crate::{UNDERLINE_COUNT, prelude::*};
use anylm::Message;
use colored::*;
use ovsy_shared::{Chunk, UserQuery};
use reqwest::Client;
use std::io::{self, Write};

/// Helper function to print markdown chunks
fn print_chunk(
    chunk: &str,
    in_code: &mut bool,
    backtick_count: &mut usize,
    is_header: &mut bool,
    is_bold: &mut bool,
    is_italic: &mut bool,
    start_of_line: &mut bool,
    is_inline_code: &mut bool,
    minus_count: &mut usize,
) {
    let block_code_color = Color::BrightMagenta;
    let inline_code_color = Color::AnsiColor(81);
    let header_color = Color::BrightMagenta;
    let default_color = Color::BrightWhite;
    let line_color = Color::AnsiColor(240);
    let mut star_count = 0;

    for c in chunk.chars() {
        // --- 1. BACKTICKS ---
        if c == '`' {
            *backtick_count += 1;
            continue;
        }

        if *backtick_count > 0 {
            if *backtick_count == 3 {
                *in_code = !*in_code;
                print!("{}", "```".color(block_code_color).bold());
                if !*in_code {
                    *start_of_line = false;
                }
            } else {
                *is_inline_code = !*is_inline_code;
                let b_color = if *is_header {
                    header_color
                } else {
                    inline_code_color
                };
                print!("{}", "`".color(b_color));
            }
            *backtick_count = 0;
            if c == '`' {
                continue;
            }
        }

        // --- 2. CODE BLOCK ---
        if *in_code {
            print!("{}", c.to_string().color(block_code_color));
            if c == '\n' {
                *start_of_line = true;
            }
            continue;
        }

        // --- 3. GORIZONTAL LINE ---
        if *start_of_line && c == '-' {
            *minus_count += 1;
            if *minus_count == 3 {
                print!("\r{}", "─".repeat(UNDERLINE_COUNT).color(line_color));
                *minus_count = 0;
            }
            continue;
        } else if *minus_count > 0 {
            print!("{}", "-".repeat(*minus_count));
            *minus_count = 0;
            *start_of_line = false;
        }

        // --- 4. HEADERS & STARS ---
        if *start_of_line && c == '#' {
            *is_header = true;
            continue;
        }

        if c == '*' {
            star_count += 1;
            if star_count == 2 {
                *is_bold = !*is_bold;
                star_count = 0;
            }
            continue;
        }
        if star_count == 1 {
            *is_italic = !*is_italic;
            star_count = 0;
        }

        // --- 5. FINAL PRINT ---
        match c {
            '\n' => {
                print!("\n");
                *is_header = false;
                *start_of_line = true;
                *is_bold = false;
                *is_italic = false;
                *is_inline_code = false;
            }
            _ => {
                if *is_header && *start_of_line && c == ' ' {
                    continue;
                }

                // header:
                let styled = if *is_header {
                    c.to_string().color(header_color).bold()
                }
                // code block:
                else if *is_inline_code {
                    c.to_string().color(inline_code_color)
                }
                // simple text:
                else {
                    let mut styled = c.to_string().color(default_color);
                    if *is_bold {
                        styled = styled.bold()
                    }
                    if *is_italic {
                        styled = styled.italic();
                    }
                    styled
                };

                print!("{styled}");
                if c != ' ' && c != '\t' {
                    *start_of_line = false;
                }
            }
        }
    }
}

/// Handles the `chat` command
pub async fn chat() -> Result<()> {
    let port = Settings::get().server.port;
    let client = Client::new();
    let mut messages: Vec<Message> = Vec::new();

    let dim = Color::AnsiColor(247);
    let bold_white = Color::BrightWhite;
    let cyan = Color::Cyan;

    print!("\x1b[2J\x1b[1;1H");
    println!(
        " {} {} {}",
        "💎".color(cyan),
        "Ovsy Assistant".color(bold_white).bold(),
        format!("v{}", app_version()).color(dim)
    );
    println!("{}", "─".repeat(UNDERLINE_COUNT).color(dim));
    println!(
        " {}\n",
        "Ready to assist. Type 'exit' to leave.".italic().color(dim)
    );

    let max_messages = Settings::get().assistant.max_messages * 2;
    let messages_limit = max_messages * 2;

    loop {
        print!("{} ", "  →".cyan().bold());
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();

        if input == "exit" || input == "quit" {
            break;
        }
        if input.is_empty() {
            continue;
        }

        // add new user message:
        messages.push(Message::user(vec![input.into()]));

        let response = client
            .post(str!("http://127.0.0.1:{port}/handle"))
            .json(&UserQuery {
                user_id: 0,
                messages: messages.clone(),
            })
            .send()
            .await
            .map_err(|e| str!("Request failed: {e}"))?;

        let bytes_stream = response.bytes_stream().map(|c| c.map_err(Into::into));
        let mut stream = Stream::read::<Chunk>(bytes_stream);
        let mut full_answer = String::new();
        let mut is_thinking = false;
        let mut first_chunk = true;

        // helper format variables:
        let mut in_code_block = false;
        let mut is_header = false;
        let mut is_bold = false;
        let mut is_italic = false;
        let mut start_of_line = true;
        let mut is_inline_code = false;
        let mut minus_count = 0;
        let mut backtick_count = 0;

        while let Some(chunk_result) = stream.read().await? {
            match chunk_result {
                Chunk::Think { think } => {
                    if !is_thinking {
                        print!("{} ", "● Thinking:".blue());
                        is_thinking = true;
                    }
                    print!("{}", think.blue());
                    io::stdout().flush().ok();
                }
                Chunk::Answer { answer } => {
                    if is_thinking {
                        print!("\x1b[2K\r");
                        is_thinking = false;
                    }
                    if first_chunk {
                        print!("{} ", "  →".magenta().bold());
                        first_chunk = false;
                    }

                    // print AI answer chunk:
                    print_chunk(
                        &answer,
                        &mut in_code_block,
                        &mut backtick_count,
                        &mut is_header,
                        &mut is_bold,
                        &mut is_italic,
                        &mut start_of_line,
                        &mut is_inline_code,
                        &mut minus_count,
                    );

                    io::stdout().flush().ok();
                    full_answer.push_str(&answer);
                }
                Chunk::Error { error } => {
                    if is_thinking {
                        println!();
                        is_thinking = false;
                    }
                    eprintln!("\n{} {}", "Error:".red().bold(), error);
                }
            }
        }

        println!(
            "\n{}",
            "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
        );

        // add final answer to context:
        if !full_answer.is_empty() {
            messages.push(Message::assistant(vec![full_answer.into()]));
        }

        // compression logic:
        if messages.len() >= messages_limit {
            let total_before = messages.len();

            print!(" {} ", "⚙".yellow());
            print!("{}", "Compressing context... ".dimmed());
            io::stdout().flush().ok();

            let compress_count = messages.len() - max_messages;
            let to_compress = messages[..compress_count].to_vec();

            let response = client
                .post(str!("http://127.0.0.1:{port}/compress"))
                .json(&UserQuery {
                    user_id: 0,
                    messages: to_compress.clone(),
                })
                .send()
                .await;

            match response {
                Ok(res) if res.status().is_success() => {
                    let bytes_stream = res.bytes_stream().map(|c| c.map_err(Into::into));
                    let mut stream = Stream::read::<Chunk>(bytes_stream);
                    let mut summary = String::new();

                    while let Some(chunk) = stream.read().await? {
                        if let Chunk::Answer { answer } = chunk {
                            summary.push_str(&answer);
                        }
                    }

                    if !summary.trim().is_empty() {
                        let new_msgs = messages[compress_count..].to_vec();
                        messages = vec![Message::system(vec![
                            str!("Summarized context: {}", summary.trim()).into(),
                        ])];
                        messages.extend(new_msgs);

                        let total_after = messages.len();
                        println!("{}", "Done".green());
                        println!(
                            " • {} {} {} {} {}",
                            "Context optimized:".dimmed(),
                            total_before.to_string().red(),
                            "→".dimmed(),
                            total_after.to_string().green(),
                            format!("(saved {} msgs)", total_before - total_after).color(dim)
                        );
                    } else {
                        println!("{}", "Failed (empty summary)".red());
                    }
                }
                _ => {
                    println!("{}", "Failed (server error)".red());
                }
            }
        }
    }

    Ok(())
}
