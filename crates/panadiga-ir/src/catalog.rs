//! Compressed offline All-Panasonic IR database (`panasonic_ir_catalog.dat.gz`) loader & search.

use std::io::{BufRead, BufReader, Cursor};
use std::sync::OnceLock;

use flate2::read::GzDecoder;

use super::types::{
    CatalogCommandSpec, KeyProtocol, PanasonicCatalogCodeset, PanasonicCatalogSummary,
    PanasonicCategory,
};

const EMBEDDED_CATALOG_GZ: &[u8] = include_bytes!("panasonic_ir_catalog.dat.gz");
const HEADER_V1: &str = "PANASONIC_IR_CATALOG_V1";
const DEFAULT_SEARCH_LIMIT: usize = 80;

static PARSED_CODESETS: OnceLock<Vec<PanasonicCatalogCodeset>> = OnceLock::new();

/// Returns all deduplicated Panasonic hardware codesets bundled offline (`OnceLock` cached).
pub fn all_codesets() -> &'static [PanasonicCatalogCodeset] {
    PARSED_CODESETS
        .get_or_init(parse_embedded_catalog)
        .as_slice()
}

/// Finds a codeset by exact catalog ID (e.g. `"curated_diga_eh55_eur7659"`).
#[must_use]
pub fn find_codeset(id: &str) -> Option<&'static PanasonicCatalogCodeset> {
    all_codesets().iter().find(|cs| cs.id == id)
}

/// Searches offline codesets matching all whitespace tokens within `category`, ranked by exact/curated hits.
#[must_use]
pub fn search_codesets(
    query: &str,
    category: PanasonicCategory,
    limit: Option<usize>,
) -> Vec<PanasonicCatalogSummary> {
    let q_trimmed = query.trim().to_ascii_lowercase();
    let tokens: Vec<&str> = q_trimmed.split_whitespace().collect();

    let mut ranked: Vec<(u8, &PanasonicCatalogCodeset)> = all_codesets()
        .iter()
        .filter(|cs| category == PanasonicCategory::All || cs.category == category)
        .filter(|cs| {
            if tokens.is_empty() {
                return true;
            }
            let blob = build_search_blob(cs);
            tokens.iter().all(|tok| blob.contains(tok))
        })
        .map(|cs| (rank_match(cs, &q_trimmed), cs))
        .collect();

    if !q_trimmed.is_empty() {
        ranked.sort_by_key(|(rank, _)| *rank);
    }

    let max_items = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
    let iter = ranked.into_iter().map(|(_, cs)| PanasonicCatalogSummary {
        id: cs.id.clone(),
        category: cs.category,
        primary_name: cs.models.first().cloned().unwrap_or_else(|| cs.id.clone()),
        models: cs.models.clone(),
        command_count: cs.commands.len(),
    });
    if max_items > 0 {
        iter.take(max_items).collect()
    } else {
        iter.collect()
    }
}

/// Returns `(codeset_count, total_model_alias_count)` across the entire database.
#[must_use]
pub fn catalog_stats() -> (usize, usize) {
    let all = all_codesets();
    let aliases = all.iter().map(|cs| cs.models.len()).sum();
    (all.len(), aliases)
}

fn build_search_blob(cs: &PanasonicCatalogCodeset) -> String {
    format!("{} {}", cs.models.join(" "), cs.id).to_ascii_lowercase()
}

fn rank_match(cs: &PanasonicCatalogCodeset, query_lower: &str) -> u8 {
    let curated = cs.id.starts_with("curated_");
    let exact = cs
        .models
        .iter()
        .any(|m| m.to_ascii_lowercase() == query_lower);
    let prefix = cs
        .models
        .iter()
        .any(|m| m.to_ascii_lowercase().starts_with(query_lower));
    match (curated, exact, prefix) {
        (true, true, _) | (true, _, true) => 0,
        (false, true, _) => 1,
        (false, false, true) => 2,
        (true, false, false) => 3,
        (false, false, false) => 4,
    }
}

fn parse_embedded_catalog() -> Vec<PanasonicCatalogCodeset> {
    let decoder = GzDecoder::new(Cursor::new(EMBEDDED_CATALOG_GZ));
    let mut reader = BufReader::new(decoder);
    let mut header = String::new();
    if reader.read_line(&mut header).is_err() || header.trim() != HEADER_V1 {
        return Vec::new();
    }

    let mut results = Vec::with_capacity(1800);
    let mut cur_id: Option<String> = None;
    let mut cur_cat = PanasonicCategory::Other;
    let mut cur_models: Vec<String> = Vec::new();
    let mut cur_cmds: Vec<CatalogCommandSpec> = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        let mut parts = line.split('\t');
        match parts.next() {
            Some("C") => {
                let (Some(id), Some(cat), Some(models_raw)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };
                flush_codeset(
                    &mut results,
                    cur_id.take(),
                    cur_cat,
                    &mut cur_models,
                    &mut cur_cmds,
                );
                cur_id = Some(id.to_string());
                cur_cat = PanasonicCategory::from_code(cat);
                cur_models = models_raw
                    .split('|')
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            Some("K") => {
                let (Some(name), Some(dev_h), Some(sub_h), Some(obc_h)) =
                    (parts.next(), parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };
                if let (Ok(dev), Ok(sub), Ok(obc)) = (
                    u8::from_str_radix(dev_h, 16),
                    u8::from_str_radix(sub_h, 16),
                    u8::from_str_radix(obc_h, 16),
                ) {
                    cur_cmds.push(CatalogCommandSpec {
                        name: name.to_string(),
                        protocol: KeyProtocol::Kaseikyo,
                        device: dev,
                        subdevice: sub,
                        obc,
                        pronto_hex: None,
                    });
                }
            }
            Some("P") => {
                if let (Some(name), Some(pronto)) = (parts.next(), parts.next()) {
                    cur_cmds.push(CatalogCommandSpec {
                        name: name.to_string(),
                        protocol: KeyProtocol::ProntoRaw,
                        device: 0,
                        subdevice: 0,
                        obc: 0,
                        pronto_hex: Some(pronto.to_string()),
                    });
                }
            }
            _ => {}
        }
    }
    flush_codeset(
        &mut results,
        cur_id.take(),
        cur_cat,
        &mut cur_models,
        &mut cur_cmds,
    );
    results
}

fn flush_codeset(
    out: &mut Vec<PanasonicCatalogCodeset>,
    id: Option<String>,
    category: PanasonicCategory,
    models: &mut Vec<String>,
    commands: &mut Vec<CatalogCommandSpec>,
) {
    if let Some(id_val) = id
        && !commands.is_empty()
    {
        out.push(PanasonicCatalogCodeset {
            id: id_val,
            category,
            models: std::mem::take(models),
            commands: std::mem::take(commands),
        });
    } else {
        models.clear();
        commands.clear();
    }
}
