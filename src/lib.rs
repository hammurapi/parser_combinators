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

type ParseResult<'a, Output> = Result<(&'a str, Output), ParseError>;

fn identifier(text: &str) -> ParseResult<String> {
    let mut chars = text.char_indices();

    let first_ident_char = match chars.next() {
        Some(next) => {
            if !next.1.is_alphabetic() {
                return Err(ParseError::IdentifiersFirstCharacterNotAlphabetic(0));
            }
            next.1
        }
        None => return Err(ParseError::PrematureEndOfText(0)),
    };

    let last_non_ident_char =
        chars.find(|item| !(item.1.is_alphanumeric() || item.1 == '-' || item.1 == '_'));

    let ident_string = first_ident_char.to_string();

    match last_non_ident_char {
        Some(last) => Ok((&text[last.0..], text[..last.0].to_string())),
        None => Ok((&text[first_ident_char.len_utf8()..], ident_string)),
    }
}

fn skip_white_space(text: &str) -> ParseResult<()> {
    let first_no_whitespace = text.char_indices().find(|item| !item.1.is_whitespace());

    match first_no_whitespace {
        Some(item) => Ok((&text[item.0..], ())),
        None => Ok((&text[text.len()..], ())),
    }
}

fn literal<'a>(text: &'a str, expected: &str) -> ParseResult<'a, String> {
    match text.starts_with(expected) {
        true => Ok((&text[expected.len()..], expected.to_string())),
        false => Err(ParseError::ExpectedLiteralNotFound(0, expected.to_string())),
    }
}

fn single_quoted_string(text: &str) -> ParseResult<String> {
    let start_quote_output = literal(text, "\'")?;
    let text = start_quote_output.0;

    let mut content = String::new();

    let mut char_indicies = text.char_indices();

    let last_char = loop {
        match char_indicies.next() {
            Some(next) => match next.1 {
                '\'' => break next,
                '\\' => content.push(escaped_char(&mut char_indicies)?),

                _ => content.push(next.1),
            },
            None => return Err(ParseError::PrematureEndOfText(0)),
        }
    };

    Ok((&text[(last_char.0 + 1)..], content))
}

fn escaped_char(char_indicies: &mut CharIndices) -> Result<char, ParseError> {
    match char_indicies.next() {
        Some(next_after_escape) => match next_after_escape.1 {
            '\'' | '\\' => Ok(next_after_escape.1),
            _ => Err(ParseError::UnknownEscapedSymbol(0, next_after_escape.1)),
        },
        None => Err(ParseError::PrematureEndOfText(0)),
    }
}

fn key_value_pair(text: &str) -> ParseResult<(String, Value)> {
    let key = identifier(text)?;

    let text = key.0;
    let text = skip_white_space(text)?.0;

    let equals = literal(text, "=")?;

    let text = equals.0;
    let text = skip_white_space(text)?.0;

    let value = value(text)?;

    Ok((value.0, (key.1, value.1)))
}

pub fn key_value_pairs(text: &str) -> ParseResult<Vec<(String, Value)>> {
    let text = skip_white_space(text)?.0;
    if text.is_empty() {
        return Ok((text, vec![]));
    }

    let mut key_value_pairs = vec![];

    let first_key_value_pair = key_value_pair(text)?;
    let mut text = first_key_value_pair.0;
    key_value_pairs.push(first_key_value_pair.1);

    loop {
        let previous_text = text;

        text = skip_white_space(text)?.0;
        let semicolon = match literal(text, ";") {
            Ok(output) => output,
            Err(_) => return Ok((previous_text, key_value_pairs)),
        };
        text = semicolon.0;

        text = skip_white_space(text)?.0;

        let a_key_value_pair_result = key_value_pair(text);
        if a_key_value_pair_result.is_err() {
            return Ok((semicolon.0, key_value_pairs));
        }
        let a_key_value_pair = a_key_value_pair_result.unwrap();

        text = a_key_value_pair.0;
        key_value_pairs.push(a_key_value_pair.1);
    }
}

fn object(text: &str) -> ParseResult<Vec<(String, Value)>> {
    let bracket = literal(text, "(")?;
    let text = bracket.0;

    let text = skip_white_space(text)?.0;

    let content_result = key_value_pairs(text);
    if content_result.is_err() {
        let bracket = literal(text, ")")?;
        let text = bracket.0;
        return Ok((text, vec![]));
    }
    let content = content_result.unwrap();
    let text = content.0;

    let text = skip_white_space(text)?.0;

    let bracket = literal(text, ")")?;
    let text = bracket.0;
    Ok((text, content.1))
}

fn list(text: &str) -> ParseResult<Vec<Value>> {
    let bracket = literal(text, "[")?;
    let text = bracket.0;

    let text = skip_white_space(text)?.0;

    let mut values = vec![];

    let first_value_result = value(text);
    if first_value_result.is_err() {
        let bracket = literal(text, "]")?;
        let text = bracket.0;
        return Ok((text, vec![]));
    }
    let first_value = first_value_result.unwrap();

    let mut text = first_value.0;
    values.push(first_value.1);

    loop {
        let previous_text = text;

        text = skip_white_space(text)?.0;

        let semicolon = match literal(text, ";") {
            Ok(output) => output,
            Err(_) => {
                text = previous_text;
                break;
            }
        };

        text = semicolon.0;

        text = skip_white_space(text)?.0;

        let a_value = value(text)?;
        text = a_value.0;
        values.push(a_value.1);
    }

    let text = skip_white_space(text)?.0;

    let bracket = literal(text, "]")?;
    let text = bracket.0;
    Ok((text, values))
}

fn value(text: &str) -> ParseResult<Value> {
    if let Ok(value) = single_quoted_string(text) {
        return Ok((value.0, Value::StringValue(value.1)));
    }

    if let Ok(value) = list(text) {
        return Ok((value.0, Value::ListValue(value.1)));
    }

    if let Ok(value) = object(text) {
        return Ok((value.0, Value::ObjectValue(value.1)));
    }

    Err(ParseError::NoValueFound(0))
}

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
        assert_eq!(
            output.1,
            (
                "key".to_string(),
                Value::StringValue("aßb'\\   ".to_string())
            )
        );

        let output = key_value_pair("key = 'aßb\\\'\\\\   '   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            (
                "key".to_string(),
                Value::StringValue("aßb'\\   ".to_string())
            )
        );
    }

    #[test]
    fn test_key_value_pairs() {
        let output = key_value_pairs("a='b';c='d';   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = key_value_pairs("a='b';c='d'   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = key_value_pairs("a = 'b' ; c = 'd'   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = key_value_pairs("   ").unwrap();
        assert_eq!(output.0, "");
        assert_eq!(output.1, vec![]);
    }

    #[test]
    fn test_object() {
        let output = object("()   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, vec![]);

        let output = object("(a='b';c='d')   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );

        let output = object("( a = 'b' ; c = 'd' )   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            vec![
                ("a".to_string(), Value::StringValue("b".to_string())),
                ("c".to_string(), Value::StringValue("d".to_string()))
            ]
        );
    }

    #[test]
    fn test_list() {
        let output = list("[]   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, vec![]);

        let output = list("['b';'d']   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            vec![
                Value::StringValue("b".to_string()),
                Value::StringValue("d".to_string())
            ]
        );

        let output = list("[ 'b' ; 'd' ]   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            vec![
                Value::StringValue("b".to_string()),
                Value::StringValue("d".to_string())
            ]
        );
    }

    #[test]
    fn test_value() {
        let output = value("'a'   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(output.1, Value::StringValue("a".to_string()));

        let output = value("[ 'b' ; 'd' ]   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
            Value::ListValue(vec![
                Value::StringValue("b".to_string()),
                Value::StringValue("d".to_string())
            ])
        );

        let output = value("( a = 'b' ; c = 'd' )   ").unwrap();
        assert_eq!(output.0, "   ");
        assert_eq!(
            output.1,
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
            key_value_pairs(option_string).unwrap();
            /*
            println!(
                "key_value_pairs({:?}) = {:?}",
                option_string,
                key_value_pairs(option_string).unwrap().1
            );
            */
        }
    }
}
