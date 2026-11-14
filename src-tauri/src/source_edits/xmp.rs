use super::{fail, repository::Snapshot, Result};
use quick_xml::{
    events::{BytesStart, Event},
    name::ResolveResult,
    NsReader, Writer,
};
const DC: &str = "http://purl.org/dc/elements/1.1/";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const PDF: &str = "http://ns.adobe.com/pdf/1.3/";
const LD: &str = "https://github.com/zabirarkam27/LibraDesk/ns/1.0/";
fn owned(ns: &str, name: &str) -> bool {
    (ns == LD
        && matches!(
            name,
            "Genre"
                | "ISBN"
                | "PublicationYear"
                | "ReadingStatus"
                | "Favorite"
                | "Categories"
                | "Tags"
                | "Collections"
        ))
        || (ns == DC
            && matches!(
                name,
                "title" | "creator" | "description" | "publisher" | "language" | "subject"
            ))
        || (ns == PDF && name == "Keywords")
}
fn escape(value: &str) -> String {
    quick_xml::escape::escape(value).into_owned()
}
fn list(prefix: &str, name: &str, kind: &str, values: &[String]) -> String {
    format!(
        "<{prefix}:{name}><rdf:{kind}>{}</rdf:{kind}></{prefix}:{name}>",
        values
            .iter()
            .map(|s| format!("<rdf:li>{}</rdf:li>", escape(s)))
            .collect::<String>()
    )
}
pub fn description(data: &Snapshot) -> Result<String> {
    let serialized = serde_json::to_string(data)?;
    if serialized.len() > 2 * 1024 * 1024
        || serialized
            .chars()
            .any(|c| c == '\u{fffe}' || c == '\u{ffff}')
    {
        return fail("metadata_invalid");
    }
    let texts = std::iter::once(data.title.as_str())
        .chain(data.subject.as_deref())
        .chain(data.genre.as_deref())
        .chain(data.publisher.as_deref())
        .chain(data.isbn.as_deref())
        .chain(data.language.as_deref())
        .chain(data.authors.iter().map(String::as_str))
        .chain(data.categories.iter().map(String::as_str))
        .chain(data.tags.iter().map(String::as_str))
        .chain(data.collections.iter().map(String::as_str));
    if texts.flat_map(str::chars).any(|c| {
        (c < ' ' && !matches!(c, '\t' | '\n' | '\r')) || matches!(c, '\u{fffe}' | '\u{ffff}')
    }) {
        return fail("metadata_invalid");
    }
    let mut xml = format!(
        r#"<rdf:Description rdf:about="" xmlns:rdf="{RDF}" xmlns:dc="{DC}" xmlns:pdf="{PDF}" xmlns:ld="{LD}">"#
    );
    for (key, value) in [
        ("title", data.title.as_str()),
        ("description", data.subject.as_deref().unwrap_or_default()),
    ] {
        xml.push_str(&format!(
            r#"<dc:{key}><rdf:Alt><rdf:li xml:lang="x-default">{}</rdf:li></rdf:Alt></dc:{key}>"#,
            escape(value)
        ));
    }
    xml.push_str(&list("dc", "creator", "Seq", &data.authors));
    xml.push_str(&list(
        "dc",
        "publisher",
        "Bag",
        &data.publisher.iter().cloned().collect::<Vec<_>>(),
    ));
    xml.push_str(&list(
        "dc",
        "language",
        "Bag",
        &data.language.iter().cloned().collect::<Vec<_>>(),
    ));
    let keywords = keywords(data);
    xml.push_str(&list("dc", "subject", "Bag", &keywords));
    xml.push_str(&format!(
        "<pdf:Keywords>{}</pdf:Keywords>",
        escape(&keywords.join("; "))
    ));
    for (key, value) in [
        ("Genre", data.genre.clone().unwrap_or_default()),
        ("ISBN", data.isbn.clone().unwrap_or_default()),
        (
            "PublicationYear",
            data.year.map(|n| n.to_string()).unwrap_or_default(),
        ),
        ("ReadingStatus", data.reading_status.clone()),
        ("Favorite", data.favorite.to_string()),
    ] {
        xml.push_str(&format!("<ld:{key}>{}</ld:{key}>", escape(&value)));
    }
    for (key, values) in [
        ("Categories", &data.categories),
        ("Tags", &data.tags),
        ("Collections", &data.collections),
    ] {
        xml.push_str(&list("ld", key, "Bag", values));
    }
    xml.push_str("</rdf:Description>");
    Ok(xml)
}
pub fn keywords(data: &Snapshot) -> Vec<String> {
    let mut result = Vec::new();
    for value in data.genre.iter().chain(&data.categories).chain(&data.tags) {
        if !result.contains(value) {
            result.push(value.clone());
        }
    }
    result
}
pub fn merge(existing: Option<&[u8]>, data: &Snapshot) -> Result<Vec<u8>> {
    let description = description(data)?;
    let Some(existing) = existing else {
        return Ok(format!(r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="{RDF}">{description}</rdf:RDF></x:xmpmeta>"#).into_bytes());
    };
    if existing.len() > 2 * 1024 * 1024 {
        return fail("metadata_invalid");
    }
    let text = std::str::from_utf8(existing).map_err(|_| super::Failure {
        code: "xmp_unsupported",
    })?;
    let mut reader = NsReader::from_str(text);
    let mut writer = Writer::new(Vec::new());
    let mut depth = 0usize;
    let mut skip = 0usize;
    let mut rdf = 0usize;
    loop {
        let event = reader.read_event().map_err(|_| super::Failure {
            code: "xmp_unsupported",
        })?;
        match event {
            Event::DocType(_) => return fail("xmp_unsupported"),
            Event::Start(start) => {
                depth += 1;
                if depth > 128 {
                    return fail("xmp_unsupported");
                }
                if skip > 0 {
                    skip += 1;
                    continue;
                }
                let (ns, local) = reader.resolver().resolve_element(start.name());
                let ns = match &ns {
                    ResolveResult::Bound(value) => value.as_ref(),
                    _ => "",
                };
                if owned(ns, local.as_ref()) {
                    skip = 1;
                    continue;
                }
                if ns == RDF && local.as_ref() == "RDF" {
                    rdf += 1;
                }
                let cleaned = clean_attributes(&reader, start)?;
                writer.write_event(Event::Start(cleaned))?;
            }
            Event::Empty(start) => {
                if skip > 0 {
                    continue;
                }
                let (ns, local) = reader.resolver().resolve_element(start.name());
                let ns = match &ns {
                    ResolveResult::Bound(value) => value.as_ref(),
                    _ => "",
                };
                if owned(ns, local.as_ref()) {
                    continue;
                }
                if ns == RDF && local.as_ref() == "RDF" {
                    rdf += 1;
                    let cleaned = clean_attributes(&reader, start)?;
                    writer.write_event(Event::Start(cleaned.clone()))?;
                    writer.get_mut().extend_from_slice(description.as_bytes());
                    writer.write_event(Event::End(cleaned.to_end()))?;
                } else {
                    writer.write_event(Event::Empty(clean_attributes(&reader, start)?))?;
                }
            }
            Event::End(end) => {
                depth = depth.checked_sub(1).ok_or(super::Failure {
                    code: "xmp_unsupported",
                })?;
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let (ns, local) = reader.resolver().resolve_element(end.name());
                if matches!(ns,ResolveResult::Bound(value) if value.as_ref()==RDF)
                    && local.as_ref() == "RDF"
                {
                    writer.get_mut().extend_from_slice(description.as_bytes());
                }
                writer.write_event(Event::End(end))?;
            }
            Event::Eof => break,
            other => {
                if skip == 0 {
                    writer.write_event(other)?;
                }
            }
        }
    }
    if rdf != 1 || depth != 0 {
        return fail("xmp_unsupported");
    }
    Ok(writer.into_inner())
}
fn clean_attributes(
    reader: &NsReader<&[u8]>,
    start: BytesStart<'_>,
) -> Result<BytesStart<'static>> {
    let mut cleaned = start.to_owned();
    cleaned.clear_attributes();
    for attr in start.attributes() {
        let attr = attr.map_err(|_| super::Failure {
            code: "xmp_unsupported",
        })?;
        let (ns, local) = reader.resolver().resolve_attribute(attr.key);
        let ns = match &ns {
            ResolveResult::Bound(value) => value.as_ref(),
            _ => "",
        };
        if !owned(ns, local.as_ref()) {
            cleaned.push_attribute(attr);
        }
    }
    Ok(cleaned)
}
