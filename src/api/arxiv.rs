use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ArxivEntryChild {
    Id(String),
    Title(String),
    Summary(String),
    Author(ArxivAuthor),
    Link(ArxivLink),
    Published(String),
    Updated(String),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ArxivEntry {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub authors: Vec<ArxivAuthor>,
    pub links: Vec<ArxivLink>,
    pub published: String,
}

pub async fn search_arxiv(query: &str) -> Result<Vec<ArxivEntry>> {
    let url = format!(
        "https://export.arxiv.org/api/query?search_query=all:{}&max_results=15",
        urlencoding::encode(query)
    );
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?.text().await?;
    
    let mut entries = Vec::new();
    let mut start_pos = 0;
    while let Some(s) = response[start_pos..].find("<entry>") {
        let entry_start = start_pos + s;
        if let Some(e) = response[entry_start..].find("</entry>") {
            let entry_end = entry_start + e + "</entry>".len();
            let entry_xml = &response[entry_start..entry_end];
            
            #[derive(Deserialize)]
            struct EntryWrapper {
                #[serde(rename = "$value")]
                children: Vec<ArxivEntryChild>,
            }

            match quick_xml::de::from_str::<EntryWrapper>(entry_xml) {
                Ok(wrapper) => {
                    let mut entry = ArxivEntry::default();
                    for child in wrapper.children {
                        match child {
                            ArxivEntryChild::Id(v) => entry.id = v,
                            ArxivEntryChild::Title(v) => entry.title = v,
                            ArxivEntryChild::Summary(v) => entry.summary = v,
                            ArxivEntryChild::Author(v) => entry.authors.push(v),
                            ArxivEntryChild::Link(v) => entry.links.push(v),
                            ArxivEntryChild::Published(v) => entry.published = v,
                            _ => (),
                        }
                    }
                    entries.push(entry);
                }
                Err(e) => tracing::warn!("Failed to parse individual arXiv entry: {}", e),
            }
            start_pos = entry_end;
        } else {
            break;
        }
    }
    
    Ok(entries)
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArxivAuthor {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArxivLink {
    #[serde(rename = "@href", default)]
    pub href: String,
    #[serde(rename = "@rel", default)]
    pub rel: String,
}

pub async fn fetch_bibtex(id: &str) -> Result<String> {
    // arXiv IDs can have versions like 2101.00001v1, bibtex works with just the base id usually
    // but the full id works too.
    let url = format!("https://arxiv.org/bibtex/{}", id);
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?.text().await?;
    Ok(response)
}

pub fn extract_id(arxiv_url: &str) -> String {
    arxiv_url.split('/').last().unwrap_or(arxiv_url).to_string()
}
