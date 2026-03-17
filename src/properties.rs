use std::collections::BTreeMap;

use crate::{Error, Result};

pub(crate) fn parse(input: &str, origin: &str) -> Result<BTreeMap<String, String>> {
    let mut properties = BTreeMap::new();

    for (line_number, line) in logical_lines(input) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            continue;
        }

        let (key, value) = split_entry(trimmed);
        let key = unescape(key, origin, line_number)?;
        let value = unescape(value, origin, line_number)?;
        properties.insert(key, value);
    }

    Ok(properties)
}

fn logical_lines(input: &str) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    let mut current: Option<(usize, String)> = None;

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim_end_matches('\r');

        match &mut current {
            Some((_, buffer)) => buffer.push_str(line.trim_start()),
            None => current = Some((line_number, line.to_string())),
        }

        if has_continuation(line) {
            if let Some((_, buffer)) = &mut current {
                buffer.pop();
            }
        } else if let Some(entry) = current.take() {
            result.push(entry);
        }
    }

    if let Some(entry) = current {
        result.push(entry);
    }

    result
}

fn has_continuation(line: &str) -> bool {
    let slash_count = line
        .chars()
        .rev()
        .take_while(|character| *character == '\\')
        .count();
    slash_count % 2 == 1
}

fn split_entry(line: &str) -> (&str, &str) {
    let mut separator = None;
    let mut escaped = false;

    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if character == '\\' {
            escaped = true;
            continue;
        }

        if character == '=' || character == ':' || character.is_whitespace() {
            separator = Some((index, character));
            break;
        }
    }

    let Some((separator_index, separator_char)) = separator else {
        return (line, "");
    };

    let key = &line[..separator_index];
    let mut value_start = separator_index + separator_char.len_utf8();

    if separator_char.is_whitespace() {
        value_start = skip_whitespace(line, value_start);
        if let Some(next) = line[value_start..].chars().next() {
            if next == '=' || next == ':' {
                value_start += next.len_utf8();
            }
        }
    }

    value_start = skip_whitespace(line, value_start);

    (key, &line[value_start..])
}

fn skip_whitespace(input: &str, mut index: usize) -> usize {
    while let Some(character) = input.get(index..).and_then(|value| value.chars().next()) {
        if !character.is_whitespace() {
            break;
        }
        index += character.len_utf8();
    }
    index
}

fn unescape(input: &str, origin: &str, line_number: usize) -> Result<String> {
    let mut output = String::with_capacity(input.len());
    let mut characters = input.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }

        let Some(escaped) = characters.next() else {
            output.push('\\');
            break;
        };

        match escaped {
            't' => output.push('\t'),
            'r' => output.push('\r'),
            'n' => output.push('\n'),
            'f' => output.push('\u{000C}'),
            '\\' => output.push('\\'),
            ' ' => output.push(' '),
            ':' => output.push(':'),
            '=' => output.push('='),
            '#' => output.push('#'),
            '!' => output.push('!'),
            'u' => {
                let code_point: String = characters.by_ref().take(4).collect();
                if code_point.len() != 4 {
                    return Err(Error::Properties {
                        origin: origin.to_string(),
                        reason: format!(
                            "line {line_number}: expected four hexadecimal digits after \\u"
                        ),
                    });
                }

                let parsed =
                    u32::from_str_radix(&code_point, 16).map_err(|_| Error::Properties {
                        origin: origin.to_string(),
                        reason: format!(
                            "line {line_number}: invalid unicode escape \\u{code_point}"
                        ),
                    })?;

                let character = char::from_u32(parsed).ok_or_else(|| Error::Properties {
                    origin: origin.to_string(),
                    reason: format!("line {line_number}: invalid unicode code point {code_point}"),
                })?;

                output.push(character);
            }
            other => output.push(other),
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_comments_continuations_and_unicode() {
        let text = "# comment\nserver.port=8080\napp.name=inventory-service\nwelcome.message=Hello,\\\n  World\nunicode.value=\\u0041\n";

        let parsed = parse(text, "test").expect("properties should parse");

        assert_eq!(parsed.get("server.port").unwrap(), "8080");
        assert_eq!(parsed.get("app.name").unwrap(), "inventory-service");
        assert_eq!(parsed.get("welcome.message").unwrap(), "Hello,World");
        assert_eq!(parsed.get("unicode.value").unwrap(), "A");
    }
}
