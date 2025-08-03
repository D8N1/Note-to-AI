use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use regex::Regex;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, CowStr};
use yaml_rust::{YamlLoader, Yaml};
use chrono::{DateTime, Utc, NaiveDateTime};
use crate::logger::Logger;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub path: PathBuf,
    pub title: String,
    pub content: String,
    pub plain_text: String,
    pub frontmatter: Option<Frontmatter>,
    pub links: Vec<Link>,
    pub tags: Vec<String>,
    pub headings: Vec<Heading>,
    pub blocks: Vec<Block>,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub publish: Option<bool>,
    pub custom_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub link_type: LinkType,
    pub target: String,
    pub alias: Option<String>,
    pub position: TextPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkType {
    WikiLink,        // [[Page Name]]
    WikiLinkAlias,   // [[Page Name|Alias]]
    MarkdownLink,    // [Text](url)
    EmbedLink,       // ![[Image.png]]
    ExternalLink,    // https://example.com
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub id: String,
    pub position: TextPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_type: BlockType,
    pub content: String,
    pub position: TextPosition,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockType {
    Paragraph,
    Heading(u8),
    CodeBlock(Option<String>), // language
    Quote,
    List,
    Table,
    Callout(String), // callout type
    Math,
    Embed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPosition {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub word_count: usize,
    pub char_count: usize,
    pub reading_time_minutes: usize,
    pub last_parsed: DateTime<Utc>,
    pub checksum: String,
}

pub struct ObsidianParser {
    logger: Logger,
    wikilink_regex: Regex,
    tag_regex: Regex,
    callout_regex: Regex,
    math_regex: Regex,
    embed_regex: Regex,
}

impl ObsidianParser {
    pub fn new() -> Result<Self> {
        let wikilink_regex = Regex::new(r"\[\[([^\]|]+)(\|([^\]]+))?\]\]")?;
        let tag_regex = Regex::new(r"(?:^|\s)#([a-zA-Z0-9_/-]+)")?;
        let callout_regex = Regex::new(r"^>\s*\[!(\w+)\]([+-]?)\s*(.*)")?;
        let math_regex = Regex::new(r"\$\$([^$]+)\$\$|\$([^$]+)\$")?;
        let embed_regex = Regex::new(r"!\[\[([^\]]+)\]\]")?;

        Ok(Self {
            logger: Logger::new("ObsidianParser"),
            wikilink_regex,
            tag_regex,
            callout_regex,
            math_regex,
            embed_regex,
        })
    }

    pub async fn parse_file(&self, path: &Path) -> Result<ParsedDocument> {
        let content = tokio::fs::read_to_string(path).await
            .context("Failed to read file")?;

        self.parse_content(path, &content).await
    }

    pub async fn parse_content(&self, path: &Path, content: &str) -> Result<ParsedDocument> {
        self.logger.debug(&format!("Parsing document: {}", path.display()));

        let (frontmatter, main_content) = self.extract_frontmatter(content)?;
        let title = self.extract_title(path, &frontmatter, &main_content);
        
        let links = self.extract_links(&main_content);
        let tags = self.extract_tags(&main_content, &frontmatter);
        let headings = self.extract_headings(&main_content);
        let blocks = self.extract_blocks(&main_content)?;
        let plain_text = self.extract_plain_text(&main_content);
        
        let metadata = DocumentMetadata {
            word_count: self.count_words(&plain_text),
            char_count: plain_text.len(),
            reading_time_minutes: self.estimate_reading_time(&plain_text),
            last_parsed: Utc::now(),
            checksum: self.calculate_checksum(content),
        };

        Ok(ParsedDocument {
            path: path.to_path_buf(),
            title,
            content: main_content.to_string(),
            plain_text,
            frontmatter,
            links,
            tags,
            headings,
            blocks,
            metadata,
        })
    }

    fn extract_frontmatter(&self, content: &str) -> Result<(Option<Frontmatter>, &str)> {
        if !content.starts_with("---") {
            return Ok((None, content));
        }

        let mut lines = content.lines();
        lines.next(); // Skip first "---"

        let mut yaml_content = String::new();
        let mut found_end = false;
        let mut line_count = 1;

        for line in lines {
            line_count += 1;
            if line.trim() == "---" {
                found_end = true;
                break;
            }
            yaml_content.push_str(line);
            yaml_content.push('\n');
        }

        if !found_end {
            return Ok((None, content));
        }

        let remaining_content = content.lines()
            .skip(line_count)
            .collect::<Vec<_>>()
            .join("\n");

        let frontmatter = self.parse_yaml_frontmatter(&yaml_content)?;
        Ok((Some(frontmatter), &remaining_content))
    }

    fn parse_yaml_frontmatter(&self, yaml_content: &str) -> Result<Frontmatter> {
        let docs = YamlLoader::load_from_str(yaml_content)
            .context("Failed to parse YAML frontmatter")?;

        let yaml = docs.first().cloned().unwrap_or(Yaml::Null);
        let mut frontmatter = Frontmatter {
            title: None,
            tags: Vec::new(),
            aliases: Vec::new(),
            created: None,
            modified: None,
            publish: None,
            custom_fields: HashMap::new(),
        };

        if let Yaml::Hash(hash) = yaml {
            for (key, value) in hash {
                if let Yaml::String(key_str) = key {
                    match key_str.as_str() {
                        "title" => {
                            if let Yaml::String(title) = value {
                                frontmatter.title = Some(title);
                            }
                        }
                        "tags" => {
                            frontmatter.tags = self.extract_yaml_tags(&value);
                        }
                        "aliases" => {
                            frontmatter.aliases = self.extract_yaml_strings(&value);
                        }
                        "created" => {
                            frontmatter.created = self.parse_yaml_date(&value);
                        }
                        "modified" => {
                            frontmatter.modified = self.parse_yaml_date(&value);
                        }
                        "publish" => {
                            if let Yaml::Boolean(publish) = value {
                                frontmatter.publish = Some(publish);
                            }
                        }
                        _ => {
                            // Store custom field
                            if let Ok(json_value) = self.yaml_to_json(&value) {
                                frontmatter.custom_fields.insert(key_str.clone(), json_value);
                            }
                        }
                    }
                }
            }
        }

        Ok(frontmatter)
    }

    fn extract_yaml_tags(&self, yaml: &Yaml) -> Vec<String> {
        match yaml {
            Yaml::String(tag) => vec![tag.clone()],
            Yaml::Array(arr) => {
                arr.iter()
                    .filter_map(|item| {
                        if let Yaml::String(tag) = item {
                            Some(tag.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn extract_yaml_strings(&self, yaml: &Yaml) -> Vec<String> {
        match yaml {
            Yaml::String(s) => vec![s.clone()],
            Yaml::Array(arr) => {
                arr.iter()
                    .filter_map(|item| {
                        if let Yaml::String(s) = item {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn parse_yaml_date(&self, yaml: &Yaml) -> Option<DateTime<Utc>> {
        if let Yaml::String(date_str) = yaml {
            // Try parsing various date formats
            if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
                return Some(dt.with_timezone(&Utc));
            }
            
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
                return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
            
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d") {
                return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }
        None
    }

    fn yaml_to_json(&self, yaml: &Yaml) -> Result<serde_json::Value> {
        match yaml {
            Yaml::String(s) => Ok(serde_json::Value::String(s.clone())),
            Yaml::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
            Yaml::Real(r) => {
                if let Ok(f) = r.parse::<f64>() {
                    Ok(serde_json::json!(f))
                } else {
                    Ok(serde_json::Value::String(r.clone()))
                }
            }
            Yaml::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            Yaml::Array(arr) => {
                let json_arr: Result<Vec<_>, _> = arr.iter()
                    .map(|item| self.yaml_to_json(item))
                    .collect();
                Ok(serde_json::Value::Array(json_arr?))
            }
            Yaml::Hash(hash) => {
                let mut json_obj = serde_json::Map::new();
                for (key, value) in hash {
                    if let Yaml::String(key_str) = key {
                        json_obj.insert(key_str.clone(), self.yaml_to_json(value)?);
                    }
                }
                Ok(serde_json::Value::Object(json_obj))
            }
            Yaml::Null => Ok(serde_json::Value::Null),
            _ => Ok(serde_json::Value::String(format!("{:?}", yaml))),
        }
    }

    fn extract_title(&self, path: &Path, frontmatter: &Option<Frontmatter>, content: &str) -> String {
        // Priority: frontmatter title > first heading > filename
        if let Some(fm) = frontmatter {
            if let Some(title) = &fm.title {
                return title.clone();
            }
        }

        // Look for first H1 heading
        if let Some(first_heading) = self.extract_headings(content).first() {
            if first_heading.level == 1 {
                return first_heading.text.clone();
            }
        }

        // Fall back to filename without extension
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled")
            .to_string()
    }

    fn extract_links(&self, content: &str) -> Vec<Link> {
        let mut links = Vec::new();
        let mut position = 0;

        // Extract wikilinks
        for cap in self.wikilink_regex.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let target = cap.get(1).unwrap().as_str().to_string();
            let alias = cap.get(3).map(|m| m.as_str().to_string());
            
            let link_type = if alias.is_some() {
                LinkType::WikiLinkAlias
            } else {
                LinkType::WikiLink
            };

            let text_position = self.calculate_position(content, full_match.start());
            
            links.push(Link {
                link_type,
                target,
                alias,
                position: text_position,
            });
        }

        // Extract embed links
        for cap in self.embed_regex.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let target = cap.get(1).unwrap().as_str().to_string();
            
            let text_position = self.calculate_position(content, full_match.start());
            
            links.push(Link {
                link_type: LinkType::EmbedLink,
                target,
                alias: None,
                position: text_position,
            });
        }

        // Extract markdown links using pulldown-cmark
        let parser = Parser::new(content);
        let mut current_pos = 0;
        
        for event in parser {
            match event {
                Event::Start(Tag::Link { dest_url, .. }) => {
                    let url = dest_url.to_string();
                    if url.starts_with("http") {
                        let text_position = self.calculate_position(content, current_pos);
                        links.push(Link {
                            link_type: LinkType::ExternalLink,
                            target: url,
                            alias: None,
                            position: text_position,
                        });
                    } else {
                        let text_position = self.calculate_position(content, current_pos);
                        links.push(Link {
                            link_type: LinkType::MarkdownLink,
                            target: url,
                            alias: None,
                            position: text_position,
                        });
                    }
                }
                _ => {}
            }
        }

        links
    }

    fn extract_tags(&self, content: &str, frontmatter: &Option<Frontmatter>) -> Vec<String> {
        let mut tags = HashSet::new();

        // Tags from frontmatter
        if let Some(fm) = frontmatter {
            for tag in &fm.tags {
                tags.insert(tag.clone());
            }
        }

        // Tags from content
        for cap in self.tag_regex.captures_iter(content) {
            if let Some(tag_match) = cap.get(1) {
                tags.insert(tag_match.as_str().to_string());
            }
        }

        tags.into_iter().collect()
    }

    fn extract_headings(&self, content: &str) -> Vec<Heading> {
        let mut headings = Vec::new();
        let parser = Parser::new(content);
        let mut current_heading_level = 0;
        let mut current_heading_text = String::new();
        let mut current_pos = 0;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    current_heading_level = level as u8;
                    current_heading_text.clear();
                }
                Event::End(TagEnd::Heading(_)) => {
                    if !current_heading_text.is_empty() {
                        let id = self.generate_heading_id(&current_heading_text);
                        let text_position = self.calculate_position(content, current_pos);
                        
                        headings.push(Heading {
                            level: current_heading_level,
                            text: current_heading_text.clone(),
                            id,
                            position: text_position,
                        });
                    }
                    current_heading_level = 0;
                }
                Event::Text(text) => {
                    if current_heading_level > 0 {
                        current_heading_text.push_str(&text);
                    }
                }
                _ => {}
            }
        }

        headings
    }

    fn extract_blocks(&self, content: &str) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        let parser = Parser::new(content);
        let mut current_pos = 0;
        let mut in_code_block = false;
        let mut code_lang: Option<String> = None;
        let mut current_content = String::new();

        for event in parser {
            match event {
                Event::Start(tag) => {
                    let block_type = match tag {
                        Tag::Paragraph => Some(BlockType::Paragraph),
                        Tag::Heading { level, .. } => Some(BlockType::Heading(level as u8)),
                        Tag::CodeBlock(lang) => {
                            in_code_block = true;
                            code_lang = if lang.is_empty() { None } else { Some(lang.to_string()) };
                            Some(BlockType::CodeBlock(code_lang.clone()))
                        }
                        Tag::BlockQuote => Some(BlockType::Quote),
                        Tag::List(_) => Some(BlockType::List),
                        Tag::Table(_) => Some(BlockType::Table),
                        _ => None,
                    };

                    if let Some(bt) = block_type {
                        current_content.clear();
                        // We'll add the block when we see the end tag
                    }
                }
                Event::End(tag_end) => {
                    let should_add_block = match tag_end {
                        TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::CodeBlock 
                        | TagEnd::BlockQuote | TagEnd::List(_) | TagEnd::Table => true,
                        _ => false,
                    };

                    if should_add_block && !current_content.trim().is_empty() {
                        let block_type = match tag_end {
                            TagEnd::Paragraph => BlockType::Paragraph,
                            TagEnd::Heading(level) => BlockType::Heading(level as u8),
                            TagEnd::CodeBlock => {
                                in_code_block = false;
                                BlockType::CodeBlock(code_lang.take())
                            }
                            TagEnd::BlockQuote => BlockType::Quote,
                            TagEnd::List(_) => BlockType::List,
                            TagEnd::Table => BlockType::Table,
                            _ => BlockType::Paragraph,
                        };

                        let text_position = self.calculate_position(content, current_pos);
                        
                        blocks.push(Block {
                            block_type,
                            content: current_content.trim().to_string(),
                            position: text_position,
                            metadata: None,
                        });
                    }
                }
                Event::Text(text) => {
                    current_content.push_str(&text);
                }
                Event::Code(text) => {
                    if !in_code_block {
                        current_content.push('`');
                        current_content.push_str(&text);
                        current_content.push('`');
                    } else {
                        current_content.push_str(&text);
                    }
                }
                _ => {}
            }
        }

        // Handle callouts (Obsidian-specific)
        self.extract_callout_blocks(content, &mut blocks);

        Ok(blocks)
    }

    fn extract_callout_blocks(&self, content: &str, blocks: &mut Vec<Block>) {
        for (line_idx, line) in content.lines().enumerate() {
            if let Some(cap) = self.callout_regex.captures(line) {
                let callout_type = cap.get(1).unwrap().as_str().to_string();
                let content_text = cap.get(3).map(|m| m.as_str()).unwrap_or("").to_string();
                
                let text_position = TextPosition {
                    start: 0, // Approximate
                    end: line.len(),
                    line: line_idx + 1,
                    column: 0,
                };

                blocks.push(Block {
                    block_type: BlockType::Callout(callout_type),
                    content: content_text,
                    position: text_position,
                    metadata: None,
                });
            }
        }
    }

    fn extract_plain_text(&self, content: &str) -> String {
        let parser = Parser::new(content);
        let mut plain_text = String::new();

        for event in parser {
            match event {
                Event::Text(text) => plain_text.push_str(&text),
                Event::Code(text) => plain_text.push_str(&text),
                Event::SoftBreak | Event::HardBreak => plain_text.push(' '),
                _ => {}
            }
        }

        // Remove wikilinks and tags
        let text = self.wikilink_regex.replace_all(&plain_text, "$1");
        let text = self.tag_regex.replace_all(&text, "");
        
        // Clean up extra whitespace
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn calculate_position(&self, content: &str, byte_pos: usize) -> TextPosition {
        let mut line = 1;
        let mut column = 1;
        let mut current_pos = 0;

        for ch in content.chars() {
            if current_pos >= byte_pos {
                break;
            }
            
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            
            current_pos += ch.len_utf8();
        }

        TextPosition {
            start: byte_pos,
            end: byte_pos,
            line,
            column,
        }
    }

    fn generate_heading_id(&self, text: &str) -> String {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    fn count_words(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }

    fn estimate_reading_time(&self, text: &str) -> usize {
        const AVERAGE_WPM: usize = 200;
        let word_count = self.count_words(text);
        (word_count + AVERAGE_WPM - 1) / AVERAGE_WPM // Ceiling division
    }

    fn calculate_checksum(&self, content: &str) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(content.as_bytes());
        hasher.finalize().to_string()
    }
}

impl Default for ObsidianParser {
    fn default() -> Self {
        Self::new().expect("Failed to create ObsidianParser")
    }
}
