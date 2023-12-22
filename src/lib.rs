// Source code for the blogpost: https://bodil.lol/parser-combinators/

use std::str::CharIndices;

type ParseResult<'a, Output> = Result<(&'a str, Output), String>;

fn identifier<'a>(text: &'a str) -> ParseResult<'a, String> {
    let mut chars = text.char_indices();

    let first_ident_char = match chars.next() {
        Some(next) => {
            if !next.1.is_alphabetic() {
                return Err("first char is not alphabetic!".to_string());
            }
            next.1
        }
        None => return Err("end of text".to_string()),
    };

    let last_non_ident_char =
        chars.find(|item| !(item.1.is_alphanumeric() || item.1 == '-' || item.1 == '_'));

    let ident_string = first_ident_char.to_string();

    match last_non_ident_char {
        Some(last) => Ok((&text[last.0..], text[..last.0].to_string())),
        None => Ok((&text[first_ident_char.len_utf8()..], ident_string)),
    }
}

fn skip_white_space<'a>(text: &'a str) -> ParseResult<'a, ()> {
    let first_no_whitespace = text.char_indices().find(|item| !item.1.is_whitespace());

    match first_no_whitespace {
        Some(item) => Ok((&text[item.0..], ())),
        None => Ok((&text[text.len()..], ())),
    }
}

fn literal<'a>(text: &'a str, expected: &str) -> ParseResult<'a, String> {
    match text.starts_with(expected) {
        true => Ok((&text[expected.len()..], expected.to_string())),
        false => Err(format!("'{}' not found", expected)),
    }
}

fn single_quoted_string<'a>(text: &'a str) -> ParseResult<'a, String> {
    let start_quote_output = literal(text, "\'")?;

    let mut content = String::new();

    let text = start_quote_output.0;

    let mut char_indicies = text.char_indices();

    let last_char = loop {
        match char_indicies.next() {
            Some(next) => match next.1 {
                '\'' => break next,
                '\\' => content.push(escaped_char(&mut char_indicies)?),

                _ => content.push(next.1),
            },
            None => return Err("End of text in string".to_string()),
        }
    };

    Ok((&text[(last_char.0 + 1)..], content))
}

fn key_value_pair<'a>(text: &'a str) -> ParseResult<'a, (String, String)> {
    let key = identifier(text)?;

    let text = key.0;
    let text = skip_white_space(text)?.0;

    let equals = literal(text, "=")?;

    let text = equals.0;
    let text = skip_white_space(text)?.0;

    let value = single_quoted_string(text)?;

    Ok((value.0, (key.1, value.1)))
}

fn escaped_char<'a>(char_indicies: &mut CharIndices) -> Result<char, String> {
    match char_indicies.next() {
        Some(next_after_escape) => match next_after_escape.1 {
            '\'' | '\\' => Ok(next_after_escape.1),
            _ => Err(format!("Unknown escpaped symbol '{}'", next_after_escape.1)),
        },
        None => Err("End of text after escape symbol".to_string()),
    }
}

/*

fn key_value_pairs<'a>(text: &'a str) -> ParseResult<'a, Vec<(String,
String)>> {
    let key_value_pairs = vec![];

    let text = skip_white_space(text)?.0;

    if text.is_empty() {
        return Ok((text, key_value_pairs));
    }

    let key_value_pair = key_value_pair(text)?;
    let text = key_value_pair.0;
    key_value_pairs.push(key_value_pair.1);

    loop {
    let text = skip_white_space(text)?.0;
    match literal(text, ";")? {
        _ => (),
        Err(_) => return Ok((key_value_pair.0, key_value_pairs)),
    }


}

*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_white_space() {
        let output = skip_white_space("  aßc  ").unwrap();
        assert_eq!(output.0, "aßc  ");

        let output = skip_white_space("    ").unwrap();
        assert_eq!(output.0, "");
    }

    #[test]
    fn test_identifier() {
        let output = identifier("aßc   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, "aßc".to_string());

        let output = identifier("a   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, "a".to_string());
    }

    #[test]
    fn test_literal() {
        let output = literal("aßc   ", "aßc").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, "aßc".to_string());
    }

    #[test]
    fn test_single_quoted_string() {
        let output = single_quoted_string("'aßb\\\'\\\\   '   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, "aßb'\\   ".to_string());
    }

    #[test]
    fn test_key_value_pair() {
        let output = key_value_pair("key='aßb\\\'\\\\   '   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, ("key".to_string(), "aßb'\\   ".to_string()));

        let output = key_value_pair("key = 'aßb\\\'\\\\   '   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, ("key".to_string(), "aßb'\\   ".to_string()));
    }
}
