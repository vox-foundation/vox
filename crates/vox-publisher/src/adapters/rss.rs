use crate::contract::NewsSiteConfig;
use crate::types::UnifiedNewsItem;
use anyhow::{Context, Result};
use std::fs;

pub async fn update_feed(item: &UnifiedNewsItem, site: &NewsSiteConfig) -> Result<()> {
    let feed_path = site.rss_feed_path.clone();

    let pub_date = item.published_at.to_rfc2822();
    let link = site.news_item_link(&item.id);

    let new_item_xml = format!(
        r#"
  <item>
    <title>{}</title>
    <link>{}</link>
    <guid isPermaLink="true">{}</guid>
    <description><![CDATA[{}]]></description>
    <pubDate>{}</pubDate>
  </item>
"#,
        xml_escape_minimal(&item.title),
        link,
        link,
        item.content_markdown,
        pub_date
    );

    if !feed_path.exists() {
        tracing::warn!("Feed file missing, creating a new one: {:?}", feed_path);
        let self_link = site.feed_self_link();
        let base = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
  <title>Vox Language Updates</title>
  <link>{0}/</link>
  <description>Changelog, release notes, and documentation updates.</description>
  <language>en-us</language>
  <lastBuildDate>{1}</lastBuildDate>
  <atom:link href="{2}" rel="self" type="application/rss+xml" />
{3}
</channel>
</rss>"#,
            site.base_url,
            pub_date,
            self_link,
            new_item_xml
        );

        if let Some(p) = feed_path.parent() {
            fs::create_dir_all(p)?;
        }
        fs::write(&feed_path, base)?;
        return Ok(());
    }

    let existing = fs::read_to_string(&feed_path).context("Failed to read feed.xml")?;

    let mut reader = quick_xml::Reader::from_str(&existing);
    let mut buf = Vec::new();
    let mut writer = quick_xml::Writer::new_with_indent(Vec::new(), b' ', 2);
    let mut inserted = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::Start(ref e))
                if e.name().as_ref() == b"item" && !inserted =>
            {
                writer.write_event(quick_xml::events::Event::Text(
                    quick_xml::events::BytesText::from_escaped(new_item_xml.as_str()),
                ))?;
                inserted = true;
                writer.write_event(quick_xml::events::Event::Start(e.clone()))?;
            }
            Ok(quick_xml::events::Event::End(ref e))
                if e.name().as_ref() == b"channel" && !inserted =>
            {
                writer.write_event(quick_xml::events::Event::Text(
                    quick_xml::events::BytesText::from_escaped(new_item_xml.as_str()),
                ))?;
                inserted = true;
                writer.write_event(quick_xml::events::Event::End(e.clone()))?;
            }
            Ok(event) => {
                writer.write_event(event)?;
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "XML Parsing error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ));
            }
        }
        buf.clear();
    }

    if inserted {
        let updated = String::from_utf8(writer.into_inner())?;
        fs::write(&feed_path, updated).context("Failed to write updated XML")?;
        tracing::info!("Successfully injected RSS item using quick-xml.");
    } else {
        tracing::error!("Failed to find <item> or </channel> tags to inject into RSS feed.");
    }

    Ok(())
}

fn xml_escape_minimal(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
