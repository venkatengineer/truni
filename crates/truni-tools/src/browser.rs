use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

pub struct BrowserTool {
    client: Client,
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0")
            .build()
            .unwrap_or_default();

        Self { client }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<(String, String)>> {
        // Use DuckDuckGo HTML search
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
        let resp = self.client.get(&url).send().await?.text().await?;

        let mut results = Vec::new();
        // Parse result snippets and titles from html
        for part in resp.split("class=\"result__snippet\"") {
            if let Some(snippet_end) = part.find("</a>") {
                let snippet = &part[..snippet_end];
                let clean_snippet = strip_html_tags(snippet);
                if !clean_snippet.is_empty() {
                    results.push((query.to_string(), clean_snippet));
                    if results.len() >= 5 {
                        break;
                    }
                }
            }
        }

        if results.is_empty() {
            results.push((query.to_string(), format!("No direct web search snippets found for '{}'", query)));
        }

        Ok(results)
    }

    pub async fn fetch_page_text(&self, url: &str) -> Result<String> {
        let resp = self.client.get(url).send().await
            .with_context(|| format!("Failed to fetch URL {}", url))?
            .text().await?;

        let text = strip_html_tags(&resp);
        let trimmed: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        let max_text = trimmed.join("\n");
        let truncated: String = max_text.chars().take(4000).collect();
        Ok(truncated)
    }
}

fn strip_html_tags(html: &str) -> String {
    let mut in_tag = false;
    let mut out = String::with_capacity(html.len());
    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
            out.push(' ');
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}
