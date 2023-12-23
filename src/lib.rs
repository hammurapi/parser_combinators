// Source code for the blogpost: https://bodil.lol/parser-combinators/

use std::str::CharIndices;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    StringValue(String),
    ListValue(Vec<Value>),
    ObjectValue(Vec<(String, Value)>),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Identifiers first character is not alphabetic! Position `{0}`!")]
    IdentifiersFirstCharacterNotAlphabetic(usize),
    #[error("Premature end of text! Position `{0}`!")]
    PrematureEndOfText(usize),
    #[error("Expected literal '{1}' not found! Position `{0}`!")]
    ExpectedLiteralNotFound(usize, String),
    #[error("Unknown excaped symbol '{1}'! Position `{0}`!")]
    UnknownEscapedSymbol(usize, char),
    #[error("No value found! Position `{0}`!")]
    NoValueFound(usize),
}

type ParseResult<'a, Output> = Result<(&'a str, usize, Output), ParseError>;

fn identifier(text: &str, position: usize) -> ParseResult<String> {
    let mut chars = text.char_indices();

    let first_ident_char = match chars.next() {
        Some(next) => {
            if !next.1.is_alphabetic() {
                return Err(ParseError::IdentifiersFirstCharacterNotAlphabetic(0));
            }
            next.1
        }
        None => return Err(ParseError::PrematureEndOfText(position)),
    };

    let last_non_ident_char =
        chars.find(|item| !(item.1.is_alphanumeric() || item.1 == '-' || item.1 == '_'));

    let ident_string = first_ident_char.to_string();

    match last_non_ident_char {
        Some(last) => Ok((
            &text[last.0..],
            position + last.0,
            text[..last.0].to_string(),
        )),
        None => Ok((
            &text[first_ident_char.len_utf8()..],
            position + first_ident_char.len_utf8(),
            ident_string,
        )),
    }
}

fn skip_white_space(text: &str, position: usize) -> ParseResult<()> {
    let first_no_whitespace = text.char_indices().find(|item| !item.1.is_whitespace());

    match first_no_whitespace {
        Some(item) => Ok((&text[item.0..], position + item.0, ())),
        None => Ok((&text[text.len()..], position + text.len(), ())),
    }
}

fn literal<'a>(text: &'a str, position: usize, expected: &str) -> ParseResult<'a, String> {
    match text.starts_with(expected) {
        true => Ok((
            &text[expected.len()..],
            position + expected.len(),
            expected.to_string(),
        )),
        false => Err(ParseError::ExpectedLiteralNotFound(
            position,
            expected.to_string(),
        )),
    }
}

fn single_quoted_string(text: &str, position: usize) -> ParseResult<String> {
    let start_quote_output = literal(text, position, "\'")?;
    let (text, position, _) = start_quote_output;

    let mut content = String::new();

    let mut char_indicies = text.char_indices();

    let mut err_position = position;
    let last_char = loop {
        let next_char = char_indicies.next();
        match next_char {
            Some(next) => match next.1 {
                '\'' => break next,
                '\\' => content.push(escaped_char(&mut char_indicies, position + next.0)?),

                _ => content.push(next.1),
            },
            None => return Err(ParseError::PrematureEndOfText(err_position)),
        }
        err_position = position + next_char.unwrap().0;
    };

    Ok((&text[(last_char.0 + 1)..], position + last_char.0, content))
}

fn escaped_char(char_indicies: &mut CharIndices, position: usize) -> Result<char, ParseError> {
    match char_indicies.next() {
        Some(next_after_escape) => match next_after_escape.1 {
            '\'' | '\\' => Ok(next_after_escape.1),
            _ => Err(ParseError::UnknownEscapedSymbol(
                position,
                next_after_escape.1,
            )),
        },
        None => Err(ParseError::PrematureEndOfText(position)),
    }
}

fn key_value_pair(text: &str, position: usize) -> ParseResult<(String, Value)> {
    let key = identifier(text, position)?;

    let (text, position, _) = key;
    let (text, position, _) = skip_white_space(text, position)?;

    let equals = literal(text, position, "=")?;

    let (text, position, _) = equals;
    let (text, position, _) = skip_white_space(text, position)?;

    let value = value(text, position)?;

    Ok((value.0, value.1, (key.2, value.2)))
}

fn key_value_pairs(text: &str, position: usize) -> ParseResult<Vec<(String, Value)>> {
    let (text, position, _) = skip_white_space(text, position)?;
    if text.is_empty() {
        return Ok((text, position, vec![]));
    }

    let mut key_value_pairs = vec![];

    let first_key_value_pair = key_value_pair(text, position)?;
    let mut text = first_key_value_pair.0;
    let mut position = first_key_value_pair.1;
    key_value_pairs.push(first_key_value_pair.2);

    loop {
        let previous_text = text;
        let previous_position = position;

        (text, position, _) = skip_white_space(text, position)?;
        let semicolon = match literal(text, position, ";") {
            Ok(output) => output,
            Err(_) => return Ok((previous_text, previous_position, key_value_pairs)),
        };
        (text, position, _) = semicolon;

        (text, position, _) = skip_white_space(text, position)?;

        let a_key_value_pair_result = key_value_pair(text, position);
        if a_key_value_pair_result.is_err() {
            return Ok((semicolon.0, semicolon.1, key_value_pairs));
        }
        let a_key_value_pair = a_key_value_pair_result.unwrap();

        (text, position, _) = a_key_value_pair;
        key_value_pairs.push(a_key_value_pair.2);
    }
}

fn object(text: &str, position: usize) -> ParseResult<Vec<(String, Value)>> {
    let bracket = literal(text, position, "(")?;
    let (text, position, _) = bracket;

    let (text, position, _) = skip_white_space(text, position)?;

    let content_result = key_value_pairs(text, position);
    if content_result.is_err() {
        let (text, position, _) = literal(text, position, ")")?;
        return Ok((text, position, vec![]));
    }
    let content = content_result.unwrap();
    let (text, position, _) = content;

    let (text, position, _) = skip_white_space(text, position)?;

    let (text, position, _) = literal(text, position, ")")?;

    Ok((text, position, content.2))
}

fn list(text: &str, position: usize) -> ParseResult<Vec<Value>> {
    let (text, position, _) = literal(text, position, "[")?;

    let (text, position, _) = skip_white_space(text, position)?;

    let mut values = vec![];

    let first_value_result = value(text, position);
    if first_value_result.is_err() {
        let (text, position, _) = literal(text, position, "]")?;

        return Ok((text, position, vec![]));
    }
    let first_value = first_value_result.unwrap();

    let mut text = first_value.0;
    let mut position = first_value.1;
    values.push(first_value.2);

    loop {
        let previous_text = text;
        let previous_position = position;

        (text, position, _) = skip_white_space(text, position)?;

        let semicolon = match literal(text, position, ";") {
            Ok(output) => output,
            Err(_) => {
                text = previous_text;
                position = previous_position;
                break;
            }
        };

        (text, position, _) = semicolon;

        (text, position, _) = skip_white_space(text, position)?;

        let a_value = value(text, position)?;
        (text, position, _) = a_value;
        values.push(a_value.2);
    }

    let (text, position, _) = skip_white_space(text, position)?;

    let (text, position, _) = literal(text, position, "]")?;
    Ok((text, position, values))
}

fn value(text: &str, position: usize) -> ParseResult<Value> {
    if let Ok(value) = single_quoted_string(text, position) {
        return Ok((value.0, value.1, Value::StringValue(value.2)));
    }

    if let Ok(value) = list(text, position) {
        return Ok((value.0, value.1, Value::ListValue(value.2)));
    }

    if let Ok(value) = object(text, position) {
        return Ok((value.0, value.1, Value::ObjectValue(value.2)));
    }

    Err(ParseError::NoValueFound(position))
}

pub fn parse_option_string(text: &str) -> Result<(&str, Vec<(String, Value)>), ParseError> {
    key_value_pairs(text, 0).map(|output| (output.0, output.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_white_space() {
        let output = skip_white_space("  aßc  ", 0).unwrap();
        assert_eq!(output.0, "aßc  ");

        let output = skip_white_space("    ", 0).unwrap();
        assert_eq!(output.0, "");
    }

    #[test]
    fn test_identifier() {
        let output = identifier("aßc   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.2, "aßc".to_string());

        let output = identifier("a   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.2, "a".to_string());
    }

    #[test]
    fn test_literal() {
        let output = literal("aßc   ", 0, "aßc").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.2, "aßc".to_string());
    }

    #[test]
    fn test_single_quoted_string() {
        let output = single_quoted_string("'aßb\\\'\\\\   '   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.2, "aßb'\\   ".to_string());
    }

    #[test]
    fn test_key_value_pair() {
        let output = key_value_pair("key='aßb\\\'\\\\   '   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            (
                "key".to_string(),
                Value::StringValue("aßb'\\   ".to_string())
            )
        );

        let output = key_value_pair("key = 'aßb\\\'\\\\   '   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            (
                "key".to_string(),
                Value::StringValue("aßb'\\   ".to_string())
            )
        );
    }

    #[test]
    fn test_key_value_pairs() {
        let output = key_value_pairs("a='b';c='d';   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = key_value_pairs("a='b';c='d'   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = key_value_pairs("a = 'b' ; c = 'd'   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = key_value_pairs("   ", 0).unwrap();
        assert_eq!(output.0, "");
        assert_eq!(output.2, vec![]);
    }

    #[test]
    fn test_object() {
        let output = object("()   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.2, vec![]);

        let output = object("(a='b';c='d')   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = object("( a = 'b' ; c = 'd' )   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );
    }

    #[test]
    fn test_list() {
        let output = list("[]   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.2, vec![]);

        let output = list("['b';'d']   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            vec![
                Value::StringValue("b".to_string()),
                Value::StringValue("d".to_string())
            ]
        );

        let output = list("[ 'b' ; 'd' ]   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            vec![
                Value::StringValue("b".to_string()),
                Value::StringValue("d".to_string())
            ]
        );
    }

    #[test]
    fn test_value() {
        let output = value("'a'   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.2, Value::StringValue("a".to_string()));

        let output = value("[ 'b' ; 'd' ]   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            Value::ListValue(vec![
                Value::StringValue("b".to_string()),
                Value::StringValue("d".to_string())
            ])
        );

        let output = value("( a = 'b' ; c = 'd' )   ", 0).unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.2,
            Value::ObjectValue(vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ])
        );
    }

    #[test]
    fn test_valid_option_strings() {
        let valid_option_strings = vec![
		"logging=(filePath='test.path')",
		"logging=(filePath='test.path');",
		"",
		" ",
		"  ",
		"key=''",
		"key='value'",
		" key = 'value' ",
		"  key = 'value';  key2 = ''  ",
		"key='\''",
		"key='\'';key2=''",
		"key=()",
		"key=(key='value')",
		"key=(key='';key2='';key3='')",
		"key=[]",
		"key=[()]",
		"key=[();();()]",
		"key=[(key='');(key='');(key='')]",
		"key=[(key='');(key=[(key='');(key='');(key='')]);(key='')]",
		"key=[(key='';key2=();key3=[]);(key=[(key='');(key='');(key='')]);(key=(key=(key=())))]",
		"rdfPath='test.path'",
		"RdfPath='test.path'",
		"logging=()",
		"logging=(filePath='')",
		"logging=(filePath='';maxFiles='10';maxFileSize='1024')",
		"modules=[]",
		"modules=[(name='test';type='doip')]",
		"modules=[(name='test';type='doip';options=(PreselectionMode='None';CombinationMode='DoIP-Group';VendorMode='Daimler';VehicleDiscoveryTime='100';Udp13401='true'))]",
		"modules=[(name='test1';type='doip';options=(PreselectionMode='None';CombinationMode='DoIP-Group'));(name='test2';type='doip';options=(PreselectionMode='None';CombinationMode='DoIP-Group'))]",
		"rdfPath='pdu_api_root_TEST.xml';logging=(filePath='sidis-pduapi.log';logLevel='info';maxFileSize='1048576';maxFiles='10');modules=[(name='localDoIpModule';type='doip';options=(PreselectionMode='None';CombinationMode='DoIP-Group';VendorMode='Daimler';VehicleDiscoveryTime='100';Udp13401='true'))]",
		"rdfPath='pdu_api_root_TEST.xml';logging =(filePath= 'sidis-pduapi.log';logLevel = 'info';maxFileSize   =  '1048576';maxFiles=   '10');modules=[(name   ='localDoIpModule';type    =    'doip';options=(PreselectionMode  = 'None';CombinationMode='DoIP-Group';VendorMode='Daimler';VehicleDiscoveryTime='100';Udp13401='true'))]",
		" rdfPath =  'pdu_api_root_TEST.xml' ;  logging =( filePath= 'sidis-pduapi.log' ;logLevel = 'info';maxFileSize   =  '1048576';maxFiles=   '10'  ;  ); modules   =[  (  name   ='localDoIpModule';type    =    'doip';options=(PreselectionMode  = 'None' ; CombinationMode='DoIP-Group' ;VendorMode='Daimler'; VehicleDiscoveryTime='100'  ;  Udp13401='true' ; ) ;); ];  ",
    ];

        for option_string in valid_option_strings {
            parse_option_string(option_string).unwrap();
            /*
            println!(
                "parse_option_string({:?}) = {:?}",
                option_string,
                parse_option_string(option_string).unwrap().1
            );
             */
        }
    }
}
