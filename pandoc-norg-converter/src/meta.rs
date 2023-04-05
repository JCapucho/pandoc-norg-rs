use crate::Builder;
use pandoc_types::definition::MetaValue;
use std::collections::HashMap;

impl<'builder, 'source> Builder<'builder, 'source> {
    pub(crate) fn handle_document_meta_block(&mut self, parameters: &[&str]) {
        if !parameters.is_empty() {
            log::warn!(
                "Embed block expected 0 parameter received: {}",
                parameters.len()
            );
            log::warn!("Extra parameters: {:?}", parameters);
        }

        let text = self
            .cursor
            .node()
            .utf8_text(self.source.as_bytes())
            .expect("Invalid text");

        let (meta, _) = parse_object_inner(text);
        self.document.extend_meta(meta);
    }
}

fn parse_object_inner(mut text: &str) -> (HashMap<String, MetaValue>, &str) {
    let mut map = HashMap::default();

    loop {
        let (name, value, rest) = parse_object_entry(text);

        map.insert(name, value);

        text = consume_whitespace(rest);
        let mut chars = text.chars();

        if let Some('}') | None = chars.next() {
            break;
        }
    }

    (map, text)
}

fn parse_object_entry(text: &str) -> (String, MetaValue, &str) {
    let rest = consume_whitespace(text);
    let (name, mut rest) = parse_string(rest);
    let mut chars = rest.chars();

    match chars.next() {
        Some(':') => rest = chars.as_str(),
        _ => {
            // TODO: Error
            log::warn!("INTERNAL: Expected colon");
            #[cfg(test)]
            panic!()
        }
    };

    let rest = consume_whitespace(rest);
    let (value, rest) = parse_value(rest);

    (name.to_string(), value, rest)
}

fn parse_value(text: &str) -> (MetaValue, &str) {
    let mut chars = text.chars();

    match chars.next() {
        Some('{') => {
            let (map, mut rest) = parse_object_inner(chars.as_str());
            let mut chars = rest.chars();

            match chars.next() {
                Some('}') => rest = chars.as_str(),
                _ => {
                    // TODO?: Error
                    log::warn!("INTERNAL: Expected closing braces");
                    #[cfg(test)]
                    panic!()
                }
            };

            (MetaValue::MetaMap(map), rest)
        }
        Some('[') => {
            let mut list = Vec::new();

            let mut rest = chars.as_str();
            loop {
                rest = consume_whitespace(rest);
                let mut chars = rest.chars();

                if let Some(']') = chars.next() {
                    rest = chars.as_str();
                    break;
                }

                let (value, new_rest) = parse_value(rest);
                list.push(value);

                rest = new_rest;
            }

            (MetaValue::MetaList(list), rest)
        }
        _ => {
            let (str, rest) = parse_string(text);
            (MetaValue::MetaString(str.to_string()), rest)
        }
    }
}

fn parse_string(text: &str) -> (&str, &str) {
    let (str, rest) = consume_any(text, |c| matches!(c, '[' | ']' | '{' | '}' | ':' | '\n'));
    (str.trim(), rest)
}

fn consume_whitespace(text: &str) -> &str {
    let (_, rest) = consume_any(text, |c| !c.is_whitespace());
    rest
}

fn consume_any(text: &str, stop: impl Fn(char) -> bool) -> (&str, &str) {
    let pos = text.find(stop).unwrap_or(text.len());
    text.split_at(pos)
}

#[cfg(test)]
mod test {
    use super::parse_object_inner;
    use pandoc_types::definition::MetaValue;
    use std::collections::HashMap;

    #[test]
    fn basic() {
        let input = r#"
            title: Look spaces
            description: This should always work
            author: brain
        "#;

        let (meta, _) = parse_object_inner(input);

        let mut expected = HashMap::default();
        expected.insert(
            "title".to_string(),
            MetaValue::MetaString("Look spaces".to_string()),
        );
        expected.insert(
            "description".to_string(),
            MetaValue::MetaString("This should always work".to_string()),
        );
        expected.insert(
            "author".to_string(),
            MetaValue::MetaString("brain".to_string()),
        );

        assert_eq!(meta, expected);
    }

    #[test]
    fn array() {
        let input = r#"
            authors: [
                Not whitespace dependent
                Take that yaml
            ]
        "#;

        let (meta, _) = parse_object_inner(input);

        let mut expected = HashMap::default();
        expected.insert(
            "authors".to_string(),
            MetaValue::MetaList(vec![
                MetaValue::MetaString("Not whitespace dependent".to_string()),
                MetaValue::MetaString("Take that yaml".to_string()),
            ]),
        );

        assert_eq!(meta, expected);
    }

    #[test]
    fn object() {
        let input = r#"
            data: {
                first: 1
                second: 2
            }
        "#;

        let (meta, _) = parse_object_inner(input);

        let mut expected = HashMap::default();
        expected.insert(
            "data".to_string(),
            MetaValue::MetaMap({
                let mut map = HashMap::default();
                map.insert("first".to_string(), MetaValue::MetaString("1".to_string()));
                map.insert("second".to_string(), MetaValue::MetaString("2".to_string()));
                map
            }),
        );

        assert_eq!(meta, expected);
    }

    #[test]
    fn object_array() {
        let input = r#"
            data: {
                first: [ 
                    ab
                    ba
                ]
                second: 2
            }
        "#;

        let (meta, _) = parse_object_inner(input);

        let mut expected = HashMap::default();
        expected.insert(
            "data".to_string(),
            MetaValue::MetaMap({
                let mut map = HashMap::default();
                map.insert(
                    "first".to_string(),
                    MetaValue::MetaList(vec![
                        MetaValue::MetaString("ab".to_string()),
                        MetaValue::MetaString("ba".to_string()),
                    ]),
                );
                map.insert("second".to_string(), MetaValue::MetaString("2".to_string()));
                map
            }),
        );

        assert_eq!(meta, expected);
    }

    #[test]
    fn nested_object() {
        let input = r#"
            data: {
                first: [ 
                    {
                        nested: array
                    }
                    ba
                ]
                second: {
                    nested: 1
                    more: { nested: 2 }
                }
            }
        "#;

        let (meta, _) = parse_object_inner(input);

        let mut expected = HashMap::default();
        expected.insert(
            "data".to_string(),
            MetaValue::MetaMap({
                let mut map = HashMap::default();
                map.insert(
                    "first".to_string(),
                    MetaValue::MetaList(vec![
                        MetaValue::MetaMap({
                            let mut map = HashMap::default();
                            map.insert(
                                "nested".to_string(),
                                MetaValue::MetaString("array".to_string()),
                            );
                            map
                        }),
                        MetaValue::MetaString("ba".to_string()),
                    ]),
                );
                map.insert(
                    "second".to_string(),
                    MetaValue::MetaMap({
                        let mut map = HashMap::default();
                        map.insert("nested".to_string(), MetaValue::MetaString("1".to_string()));
                        map.insert(
                            "more".to_string(),
                            MetaValue::MetaMap({
                                let mut map = HashMap::default();
                                map.insert(
                                    "nested".to_string(),
                                    MetaValue::MetaString("2".to_string()),
                                );
                                map
                            }),
                        );
                        map
                    }),
                );
                map
            }),
        );

        assert_eq!(meta, expected);
    }
}
