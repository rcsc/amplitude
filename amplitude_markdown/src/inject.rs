mod quiz;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag};
use std::collections::HashMap;

use crate::link_concat::LinkDefs;

type Callback = fn(Vec<Event>, &str, &mut Vec<Event>, &mut ParseState) -> anyhow::Result<()>;

pub(crate) struct ParseState<'a> {
    pub links: &'a LinkDefs<'a>,
}

/// A list of tags that are expected to be found in the markdown to call the
/// callback
#[derive(Debug)]
enum ExpectedTag {
    CodeBlock(Option<&'static str>),
    Quote,
}

impl ExpectedTag {
    fn matches(&self, tag: &Tag) -> bool {
        use ExpectedTag::*;
        match self {
            CodeBlock(Some(lang)) => {
                tag == &Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::Borrowed(lang)))
            }
            CodeBlock(None) => matches!(tag, Tag::CodeBlock(CodeBlockKind::Fenced(_))),
            Quote => matches!(tag, Tag::BlockQuote),
        }
    }
}

#[ctor::ctor]
static INJECTION_TAGS: HashMap<String, (ExpectedTag, Callback)> = {
    let mut m = HashMap::new();
    let mut insert = |tag: &str, expected: ExpectedTag, callback: Callback| {
        m.insert(tag.to_string(), (expected, callback));
    };
    use ExpectedTag::*;
    insert("quiz", CodeBlock(Some("toml")), quiz::inject_quiz);
    m
};

pub(crate) fn inject<'a>(
    parser: Parser<'a, '_>,
    links: &'a LinkDefs<'a>,
) -> anyhow::Result<Vec<Event<'a>>> {
    use pulldown_cmark::Event::*;
    use pulldown_cmark::Tag::*;

    let mut out = Vec::new();
    let mut state = ParseState { links };
    let mut iter = parser.into_iter();

    let mut s = String::new();
    let mut i = 0;

    while let Some(event) = iter.next() {
        // we want to find: [Start(Paragraph), Text(s), End(Paragraph)]
        let b = match i {
            0 => matches!(event, Start(Paragraph)),
            1 => {
                if let Text(ref str) = event && str.as_bytes()[0] == '@' as u8 {
                    s = str.to_string();
                    true
                } else {
                    false
                }
            },
            2 => matches!(event, End(Paragraph)),
            _ => unreachable!(),
        };
        if b {
            i += 1;
        }
        // wowie we have a tag
        if i == 3 {
            out.pop();
            out.pop();
            let (tag, data) = s.split_once(';').unwrap_or((&s, ""));
            // no whitespace allowed in tag or data
            if tag.trim().contains(|c: char| c.is_whitespace()) {
                anyhow::bail!("`@` tags cannot contain whitespace: {}", tag);
            }
            // look in INJECTION_TAGS for the tag & callback to call
            if let Some((expected, callback)) = INJECTION_TAGS.get(&tag.trim()[1..]) {
                let Some(event) = iter.next() else { anyhow::bail!("Unexpected end of `@` tag: {}", tag) };
                let Start(tag) = event else { anyhow::bail!("Unexpected event: {:?}", event) };
                if !expected.matches(&tag) {
                    anyhow::bail!("Did not expect tag: {:?}, expected {expected:?}", tag);
                }
                let mut buf = vec![Start(tag)];

                // consume everything until the end of the tag
                let mut i = 1;
                while i > 0 {
                    let event = iter.next().unwrap();
                    match event {
                        Start(_) => i += 1,
                        End(_) => i -= 1,
                        _ => {}
                    }
                    buf.push(event);
                }

                // call the callback
                callback(buf, data, &mut out, &mut state)?;
            } else {
                anyhow::bail!("Unknown `@` tag: {}", tag)
            }
            i = 0;
            continue;
        }

        out.push(event);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    // use std::default::default;

    // use pulldown_cmark::Parser;

    // use crate::parse::{parse_into_events, parse};

    //     #[test]
    //     fn test_inject() {
    //         let input =
    //             r#"
    // @quiz
    // ```toml
    // question = """
    // But like *are* you gay?
    // """

    // [[answers]]
    // text = "answer"
    // response = "woo"
    // ```"#;
    //         panic!("{:#?}", parse(input, &default()))
    //         // panic!("{:#?}", parse_into_events(parser, &default()).unwrap().into_iter().collect::<Vec<_>>())
    //     }
}