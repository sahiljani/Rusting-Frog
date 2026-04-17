// Parses SeoElementFilterKey.java and generates a multi-file FilterKey
// module. Built-in filters each get a variant; the seven 100-slot families
// (Custom Search, Custom Extractor, Custom JavaScript, and AI extractors
// for OpenAI / Anthropic / Gemini / Ollama — 700 Java entries total)
// collapse into four parameterized variants that hold slot data.
//
// Output layout (crates/core/src/filter_key/):
//   mod.rs             — types, enum, kind(), all(), for_tab(), Serde
//   tab.rs             — tab()
//   i18n.rs            — i18n_key()
//   filter_key_type.rs — filter_key_type()
//   severity.rs        — severity()
//   bit_pos.rs         — bit_pos()
//   watermark.rs       — has_watermark()
//   deprecated.rs      — is_deprecated()
//   display.rs         — display_name()
//   serde_key.rs       — serde_key() + from_serde_key()
//
// Usage: node scripts/gen_filter_keys.js

const fs = require('fs');
const path = require('path');

const JAVA = 'C:/Users/User/Desktop/screamingfrog1/sources/seo/spider/seoelements/SeoElementFilterKey.java';
const OUT_DIR = path.resolve(__dirname, '..', 'crates', 'core', 'src', 'filter_key');

const raw = fs.readFileSync(JAVA, 'utf8');

// Enum entry: NAME(tab, bitPos, idx, i18n, fifthArgOrNull, FilterKeyType.X, id706500854.Y),
const RX = /^\s+([A-Z][A-Z0-9_]*)\(id1377782850\.([A-Z0-9_]+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*"([^"]+)"\s*,\s*([^,]+?)\s*,\s*FilterKeyType\.([A-Z_]+)\s*,\s*id706500854\.([A-Z]+)\s*\)\s*[,;]/gm;

const TABS = {
  JAVASCRIPT: 'JavaScript',
  H1: 'H1',
  H2: 'H2',
  INTERNAL: 'Internal',
  EXTERNAL: 'External',
  AMP: 'Amp',
  CANONICALS: 'Canonicals',
  CONTENT: 'Content',
  CUSTOM_EXTRACTION: 'CustomExtraction',
  CUSTOM_SEARCH: 'CustomSearch',
  CUSTOM_JAVASCRIPT: 'CustomJavaScript',
  DIRECTIVES: 'Directives',
  ANALYTICS: 'Analytics',
  SEARCH_CONSOLE: 'SearchConsole',
  HREFLANG: 'Hreflang',
  IMAGES: 'Images',
  LINK_METRICS: 'LinkMetrics',
  META_DESCRIPTION: 'MetaDescription',
  META_KEYWORDS: 'MetaKeywords',
  PAGE_SPEED: 'PageSpeed',
  PAGINATION: 'Pagination',
  RESPONSE_CODE: 'ResponseCode',
  SECURITY: 'Security',
  SITEMAPS: 'Sitemaps',
  STRUCTURED_DATA: 'StructuredData',
  PAGE_TITLES: 'PageTitles',
  URL: 'Url',
  PARITY: 'Parity',
  LINKS: 'Links',
  VALIDATION: 'Validation',
  MOBILE: 'Mobile',
  AI: 'Ai',
  ACCESSIBILITY: 'Accessibility',
  UNDEF: 'Undef',
};

const FILTER_KEY_TYPES = {
  NORMAL: 'Normal',
  POST_CRAWL_ANALYSIS: 'PostCrawlAnalysis',
  SPELLING_AND_GRAMMAR: 'SpellingAndGrammar',
};

const COMPOUND = {
  JAVASCRIPT: 'JavaScript', HTML: 'Html', CSS: 'Css', URL: 'Url', URLS: 'Urls',
  HTTP: 'Http', HTTPS: 'Https', PDF: 'Pdf', XML: 'Xml', AMP: 'Amp', JSON: 'Json',
  RSS: 'Rss', API: 'Api', GZIP: 'Gzip', GA: 'Ga', GSC: 'Gsc',
  LCP: 'Lcp', FCP: 'Fcp', CLS: 'Cls', FID: 'Fid', TTI: 'Tti', TTFB: 'Ttfb',
  INP: 'Inp', TBT: 'Tbt', SEO: 'Seo', AI: 'Ai', UUID: 'Uuid', ARIA: 'Aria',
  AVIF: 'Avif', WEBP: 'Webp',
};

function toCamel(name) {
  return name
    .split('_')
    .map((p) => {
      if (!p) return '';
      if (COMPOUND[p]) return COMPOUND[p];
      if (/^[0-9]/.test(p)) return p;
      return p[0] + p.slice(1).toLowerCase();
    })
    .join('');
}

const toSnake = (name) => name.toLowerCase();

function humanize(i18n) {
  const leaf = i18n.split('.').pop();
  return leaf
    .split('_')
    .map((w) => {
      if (w === 'x') return 'X';
      if (w.length <= 3 && w === w.toLowerCase()) {
        const keep = new Set(['is', 'of', 'to', 'in', 'on', 'at', 'by', 'or', 'as']);
        if (keep.has(w)) return w;
      }
      return w[0].toUpperCase() + w.slice(1);
    })
    .join(' ');
}

const SLOT_FAMILIES = [
  { prefix: 'CUSTOM_FILTER',     variant: 'CustomSearchSlot',     tab: 'CustomSearch',     i18n: 'tab.custom_search.filter',         provider: null },
  { prefix: 'CUSTOM_EXTRACTOR',  variant: 'CustomExtractorSlot',  tab: 'CustomExtraction', i18n: 'tab.custom_extraction.extractor',  provider: null },
  { prefix: 'CUSTOM_JAVASCRIPT', variant: 'CustomJavaScriptSlot', tab: 'CustomJavaScript', i18n: 'tab.custom_javascript.extractor',  provider: null },
  { prefix: 'OPENAI_FILTER',     variant: 'AiSlot',               tab: 'Ai',               i18n: 'tab.openai.prompt',                provider: 'Openai' },
  { prefix: 'ANTHROPIC_FILTER',  variant: 'AiSlot',               tab: 'Ai',               i18n: 'tab.anthropic.prompt',             provider: 'Anthropic' },
  { prefix: 'GEMINI_FILTER',     variant: 'AiSlot',               tab: 'Ai',               i18n: 'tab.gemini.prompt',                provider: 'Gemini' },
  { prefix: 'OLLAMA_FILTER',     variant: 'AiSlot',               tab: 'Ai',               i18n: 'tab.ollama.prompt',                provider: 'Ollama' },
];

function slotFamilyFor(name) {
  for (const f of SLOT_FAMILIES) {
    const m = name.match(new RegExp(`^${f.prefix}_(\\d+)$`));
    if (m) return { family: f, n: Number(m[1]) };
  }
  return null;
}

const ALL_RENAMES = {
  CUSTOM_FILTER_ALL:     'CustomSearchAll',
  CUSTOM_EXTRACTION_ALL: 'CustomExtractorAll',
  CUSTOM_JAVASCRIPT_ALL: 'CustomJavaScriptAll',
  AI_FILTER_ALL:         'AiAll',
};
const ALL_SERDE_RENAMES = {
  CUSTOM_FILTER_ALL:     'custom_search_all',
  CUSTOM_EXTRACTION_ALL: 'custom_extractor_all',
  CUSTOM_JAVASCRIPT_ALL: 'custom_javascript_all',
  AI_FILTER_ALL:         'ai_all',
};

// ---- Parse ----
const fixed = [];
const slots = {};
let m;
while ((m = RX.exec(raw)) !== null) {
  const [, name, tab, bitPos, idx, i18n, fifthArg, fkt, sev] = m;
  if (!TABS[tab]) throw new Error(`Unknown tab: ${tab} in ${name}`);
  const fam = slotFamilyFor(name);
  if (fam) {
    (slots[fam.family.prefix] ||= []).push({
      n: fam.n,
      bitPos: Number(bitPos),
      fkt,
      sev,
    });
    continue;
  }
  const variantName = ALL_RENAMES[name] || toCamel(name);
  const serdeName = ALL_SERDE_RENAMES[name] || toSnake(name);
  fixed.push({
    name,
    variant: variantName,
    serde: serdeName,
    tab,
    tabRust: TABS[tab],
    bitPos: Number(bitPos),
    idx: Number(idx),
    i18n,
    fkt,
    fktRust: FILTER_KEY_TYPES[fkt],
    sev,
    severity: sev === 'ISSUE' ? 'Issue' : 'Stat',
    hasWatermark: fifthArg.trim() !== 'null',
    displayName: humanize(i18n),
    isDeprecated: /_DEPRECATED($|_)/.test(name),
  });
}

if (fixed.length < 400) throw new Error(`Only parsed ${fixed.length} fixed entries — regex broke`);

const slotBase = {};
for (const f of SLOT_FAMILIES) {
  const list = slots[f.prefix] || [];
  if (list.length !== 100) throw new Error(`${f.prefix} had ${list.length} entries, expected 100`);
  list.sort((a, b) => a.n - b.n);
  slotBase[f.prefix] = list[0].bitPos - 1;
  const uniqFkt = new Set(list.map((e) => e.fkt));
  const uniqSev = new Set(list.map((e) => e.sev));
  if (uniqFkt.size !== 1) throw new Error(`${f.prefix} mixed fkt: ${[...uniqFkt]}`);
  if (uniqSev.size !== 1) throw new Error(`${f.prefix} mixed sev: ${[...uniqSev]}`);
}

const seen = new Set();
for (const e of fixed) {
  if (seen.has(e.variant)) throw new Error(`Duplicate variant: ${e.variant} (from ${e.name})`);
  seen.add(e.variant);
}

const byTab = {};
for (const e of fixed) (byTab[e.tab] ||= []).push(e);

const slotFkt = {};
const slotSev = {};
for (const f of SLOT_FAMILIES) {
  slotFkt[f.prefix] = FILTER_KEY_TYPES[slots[f.prefix][0].fkt];
  slotSev[f.prefix] = slots[f.prefix][0].sev === 'ISSUE' ? 'Issue' : 'Stat';
}

const TAB_ORDER = [
  'INTERNAL', 'EXTERNAL', 'PAGE_TITLES', 'META_DESCRIPTION', 'META_KEYWORDS',
  'H1', 'H2', 'IMAGES', 'CANONICALS', 'PAGINATION', 'DIRECTIVES', 'HREFLANG',
  'JAVASCRIPT', 'AMP', 'LINKS', 'RESPONSE_CODE', 'URL', 'CONTENT', 'SECURITY',
  'SITEMAPS', 'STRUCTURED_DATA', 'MOBILE', 'VALIDATION', 'PAGE_SPEED',
  'ANALYTICS', 'SEARCH_CONSOLE', 'LINK_METRICS', 'PARITY',
  'CUSTOM_SEARCH', 'CUSTOM_EXTRACTION', 'CUSTOM_JAVASCRIPT', 'AI',
  'ACCESSIBILITY', 'UNDEF',
];

// ---- Emit files ----

const HEADER = (module) =>
  `// AUTO-GENERATED by scripts/gen_filter_keys.js from\n` +
  `// screamingfrog1/sources/seo/spider/seoelements/SeoElementFilterKey.java\n` +
  `// Do not edit by hand — regenerate with \`node scripts/gen_filter_keys.js\`.\n` +
  `//\n` +
  `// This is ${module}. The enum itself and shared types live in \`mod.rs\`;\n` +
  `// each metadata method lives in its own submodule for readability.\n\n`;

function emitModRs() {
  let s = '';
  s += `// AUTO-GENERATED by scripts/gen_filter_keys.js from\n`;
  s += `// screamingfrog1/sources/seo/spider/seoelements/SeoElementFilterKey.java\n`;
  s += `// Do not edit by hand — regenerate with \`node scripts/gen_filter_keys.js\`.\n`;
  s += `//\n`;
  s += `// The 700 user-configurable slot filters in the Java source\n`;
  s += `// (CUSTOM_FILTER_N, CUSTOM_EXTRACTOR_N, CUSTOM_JAVASCRIPT_N, plus\n`;
  s += `// {OPENAI,ANTHROPIC,GEMINI,OLLAMA}_FILTER_N, 100 each) are collapsed\n`;
  s += `// into four parameterized Rust variants that hold the slot number.\n`;
  s += `// That drops the enum from 1157 variants to ${fixed.length} + 4.\n`;
  s += `//\n`;
  s += `// Layout: this file defines the enum + shared types. Each metadata\n`;
  s += `// method (tab, i18n_key, severity, …) is a separate submodule with\n`;
  s += `// its own \`impl FilterKey\` block — Rust coalesces multiple impl\n`;
  s += `// blocks for the same type, so callers still see one flat API.\n\n`;
  s += `#![allow(clippy::too_many_lines)]\n\n`;
  s += `use std::fmt;\n`;
  s += `use serde::{de, Deserialize, Deserializer, Serialize, Serializer};\n\n`;
  s += `use crate::tab::TabKey;\n\n`;

  // Per-method submodules. Declared `mod`, not `pub mod` — the impl blocks\n
  // they define are attached to the public FilterKey anyway.
  s += `mod bit_pos;\n`;
  s += `mod deprecated;\n`;
  s += `mod display;\n`;
  s += `mod filter_key_type;\n`;
  s += `mod i18n;\n`;
  s += `mod serde_key;\n`;
  s += `mod severity;\n`;
  s += `mod tab;\n`;
  s += `mod watermark;\n\n`;

  s += `/// Which AI provider a user-configurable AI-extraction slot targets.\n`;
  s += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n`;
  s += `pub enum AiProvider {\n`;
  s += `    Openai,\n    Anthropic,\n    Gemini,\n    Ollama,\n}\n\n`;
  s += `impl AiProvider {\n`;
  s += `    pub const ALL: &'static [Self] = &[Self::Openai, Self::Anthropic, Self::Gemini, Self::Ollama];\n\n`;
  s += `    pub fn as_str(&self) -> &'static str {\n`;
  s += `        match self {\n`;
  s += `            Self::Openai => "openai",\n`;
  s += `            Self::Anthropic => "anthropic",\n`;
  s += `            Self::Gemini => "gemini",\n`;
  s += `            Self::Ollama => "ollama",\n`;
  s += `        }\n    }\n\n`;
  s += `    pub fn display_name(&self) -> &'static str {\n`;
  s += `        match self {\n`;
  s += `            Self::Openai => "OpenAI",\n`;
  s += `            Self::Anthropic => "Anthropic",\n`;
  s += `            Self::Gemini => "Gemini",\n`;
  s += `            Self::Ollama => "Ollama",\n`;
  s += `        }\n    }\n\n`;
  s += `    pub fn parse(s: &str) -> Option<Self> {\n`;
  s += `        match s {\n`;
  s += `            "openai" => Some(Self::Openai),\n`;
  s += `            "anthropic" => Some(Self::Anthropic),\n`;
  s += `            "gemini" => Some(Self::Gemini),\n`;
  s += `            "ollama" => Some(Self::Ollama),\n`;
  s += `            _ => None,\n`;
  s += `        }\n    }\n}\n\n`;

  s += `/// Classification of how a filter is evaluated.\n`;
  s += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n`;
  s += `#[serde(rename_all = "snake_case")]\n`;
  s += `pub enum FilterKeyType {\n`;
  s += `    Normal,\n    PostCrawlAnalysis,\n    SpellingAndGrammar,\n}\n\n`;

  s += `/// STAT = informational counter; ISSUE = flagged problem.\n`;
  s += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n`;
  s += `#[serde(rename_all = "snake_case")]\n`;
  s += `pub enum FilterSeverity {\n    Stat,\n    Issue,\n}\n\n`;

  s += `/// Semantic classification returned by [\`FilterKey::kind\`].\n`;
  s += `#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n`;
  s += `pub enum FilterKind {\n`;
  s += `    /// A fixed filter baked into the crawler (e.g. \`TitleMissing\`).\n`;
  s += `    BuiltIn,\n`;
  s += `    /// User-defined Custom Search slot, 1..=100.\n`;
  s += `    CustomSearchSlot(u8),\n`;
  s += `    /// User-defined Custom Extractor slot, 1..=100.\n`;
  s += `    CustomExtractorSlot(u8),\n`;
  s += `    /// User-defined Custom JavaScript slot, 1..=100.\n`;
  s += `    CustomJavaScriptSlot(u8),\n`;
  s += `    /// User-defined AI-extraction slot, 1..=100, per provider.\n`;
  s += `    AiSlot(AiProvider, u8),\n}\n\n`;

  s += `/// Every filter shown in the SF-style UI, across all 33 tabs.\n`;
  s += `///\n`;
  s += `/// Ported from \`seo.spider.seoelements.SeoElementFilterKey\`. Built-in\n`;
  s += `/// filters each get one variant; the four user-configurable slot families\n`;
  s += `/// collapse into parameterized variants — see [\`FilterKind\`].\n`;
  s += `///\n`;
  s += `/// Serde format: snake_case. Slot variants serialize as\n`;
  s += `/// \`custom_search_slot_<n>\`, \`custom_extractor_slot_<n>\`,\n`;
  s += `/// \`custom_javascript_slot_<n>\`, or \`ai_<provider>_slot_<n>\`.\n`;
  s += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n`;
  s += `pub enum FilterKey {\n`;
  for (const tabKey of TAB_ORDER) {
    const group = byTab[tabKey];
    if (!group || !group.length) continue;
    s += `    // ---- ${TABS[tabKey]} (${group.length} built-in) ----\n`;
    for (const e of group) s += `    ${e.variant},\n`;
    s += `\n`;
  }
  s += `    // ---- Parameterized slot filters (1..=100 each) ----\n`;
  s += `    /// User-defined Custom Search slot filter. (Java: CUSTOM_FILTER_N)\n`;
  s += `    CustomSearchSlot(u8),\n`;
  s += `    /// User-defined Custom Extractor slot filter. (Java: CUSTOM_EXTRACTOR_N)\n`;
  s += `    CustomExtractorSlot(u8),\n`;
  s += `    /// User-defined Custom JavaScript slot filter. (Java: CUSTOM_JAVASCRIPT_N)\n`;
  s += `    CustomJavaScriptSlot(u8),\n`;
  s += `    /// User-defined AI-extraction slot, per provider.\n`;
  s += `    /// (Java: {OPENAI,ANTHROPIC,GEMINI,OLLAMA}_FILTER_N)\n`;
  s += `    AiSlot(AiProvider, u8),\n}\n\n`;

  // Small root-level methods that don't warrant their own file.
  s += `impl FilterKey {\n`;
  s += `    /// Semantic grouping of this filter.\n`;
  s += `    pub fn kind(&self) -> FilterKind {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(n) => FilterKind::CustomSearchSlot(*n),\n`;
  s += `            Self::CustomExtractorSlot(n) => FilterKind::CustomExtractorSlot(*n),\n`;
  s += `            Self::CustomJavaScriptSlot(n) => FilterKind::CustomJavaScriptSlot(*n),\n`;
  s += `            Self::AiSlot(p, n) => FilterKind::AiSlot(*p, *n),\n`;
  s += `            _ => FilterKind::BuiltIn,\n`;
  s += `        }\n    }\n\n`;

  s += `    /// Every filter key, in a stable order (all fixed variants first, then slot families).\n`;
  s += `    pub fn all() -> Vec<FilterKey> {\n`;
  s += `        let mut v = Vec::with_capacity(${fixed.length} + 700);\n`;
  s += `        v.extend_from_slice(&[\n`;
  for (const e of fixed) s += `            Self::${e.variant},\n`;
  s += `        ]);\n`;
  s += `        for n in 1..=100u8 { v.push(Self::CustomSearchSlot(n)); }\n`;
  s += `        for n in 1..=100u8 { v.push(Self::CustomExtractorSlot(n)); }\n`;
  s += `        for n in 1..=100u8 { v.push(Self::CustomJavaScriptSlot(n)); }\n`;
  s += `        for p in AiProvider::ALL.iter().copied() {\n`;
  s += `            for n in 1..=100u8 { v.push(Self::AiSlot(p, n)); }\n`;
  s += `        }\n`;
  s += `        v\n    }\n\n`;

  s += `    /// All filter keys for a given tab, in declaration order.\n`;
  s += `    pub fn for_tab(tab: TabKey) -> Vec<FilterKey> {\n`;
  s += `        Self::all().into_iter().filter(|k| k.tab() == tab).collect()\n`;
  s += `    }\n}\n\n`;

  s += `// ---- Serde ----\n\n`;
  s += `impl Serialize for FilterKey {\n`;
  s += `    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {\n`;
  s += `        s.serialize_str(&self.serde_key())\n`;
  s += `    }\n}\n\n`;
  s += `impl<'de> Deserialize<'de> for FilterKey {\n`;
  s += `    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {\n`;
  s += `        struct V;\n`;
  s += `        impl<'de> de::Visitor<'de> for V {\n`;
  s += `            type Value = FilterKey;\n`;
  s += `            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {\n`;
  s += `                f.write_str("a filter_key string")\n`;
  s += `            }\n`;
  s += `            fn visit_str<E: de::Error>(self, s: &str) -> Result<FilterKey, E> {\n`;
  s += `                FilterKey::from_serde_key(s)\n`;
  s += `                    .ok_or_else(|| E::custom(format!("unknown filter_key: {s}")))\n`;
  s += `            }\n        }\n        d.deserialize_str(V)\n    }\n}\n`;
  return s;
}

function emitTab() {
  let s = HEADER('`FilterKey::tab`');
  s += `use super::FilterKey;\nuse crate::tab::TabKey;\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// Which UI tab this filter belongs to.\n`;
  s += `    pub fn tab(&self) -> TabKey {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(_) => TabKey::CustomSearch,\n`;
  s += `            Self::CustomExtractorSlot(_) => TabKey::CustomExtraction,\n`;
  s += `            Self::CustomJavaScriptSlot(_) => TabKey::CustomJavaScript,\n`;
  s += `            Self::AiSlot(_, _) => TabKey::Ai,\n`;
  for (const e of fixed) s += `            Self::${e.variant} => TabKey::${e.tabRust},\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

function emitI18n() {
  let s = HEADER('`FilterKey::i18n_key`');
  s += `use super::{AiProvider, FilterKey};\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// i18n key for the filter's display name (e.g. "tab.internal.filter.html").\n`;
  s += `    pub fn i18n_key(&self) -> &'static str {\n`;
  s += `        match self {\n`;
  for (const f of SLOT_FAMILIES) {
    if (f.variant === 'AiSlot') {
      s += `            Self::AiSlot(AiProvider::${f.provider}, _) => "${f.i18n}",\n`;
    } else {
      s += `            Self::${f.variant}(_) => "${f.i18n}",\n`;
    }
  }
  for (const e of fixed) s += `            Self::${e.variant} => "${e.i18n}",\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

function emitFilterKeyType() {
  let s = HEADER('`FilterKey::filter_key_type`');
  s += `use super::{AiProvider, FilterKey, FilterKeyType};\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// Evaluation classification: normal / post-crawl / spelling.\n`;
  s += `    pub fn filter_key_type(&self) -> FilterKeyType {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(_) => FilterKeyType::${slotFkt.CUSTOM_FILTER},\n`;
  s += `            Self::CustomExtractorSlot(_) => FilterKeyType::${slotFkt.CUSTOM_EXTRACTOR},\n`;
  s += `            Self::CustomJavaScriptSlot(_) => FilterKeyType::${slotFkt.CUSTOM_JAVASCRIPT},\n`;
  s += `            Self::AiSlot(AiProvider::Openai, _) => FilterKeyType::${slotFkt.OPENAI_FILTER},\n`;
  s += `            Self::AiSlot(AiProvider::Anthropic, _) => FilterKeyType::${slotFkt.ANTHROPIC_FILTER},\n`;
  s += `            Self::AiSlot(AiProvider::Gemini, _) => FilterKeyType::${slotFkt.GEMINI_FILTER},\n`;
  s += `            Self::AiSlot(AiProvider::Ollama, _) => FilterKeyType::${slotFkt.OLLAMA_FILTER},\n`;
  for (const e of fixed) s += `            Self::${e.variant} => FilterKeyType::${e.fktRust},\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

function emitSeverity() {
  let s = HEADER('`FilterKey::severity`');
  s += `use super::{AiProvider, FilterKey, FilterSeverity};\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// STAT = counter, ISSUE = flagged problem.\n`;
  s += `    pub fn severity(&self) -> FilterSeverity {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(_) => FilterSeverity::${slotSev.CUSTOM_FILTER},\n`;
  s += `            Self::CustomExtractorSlot(_) => FilterSeverity::${slotSev.CUSTOM_EXTRACTOR},\n`;
  s += `            Self::CustomJavaScriptSlot(_) => FilterSeverity::${slotSev.CUSTOM_JAVASCRIPT},\n`;
  s += `            Self::AiSlot(AiProvider::Openai, _) => FilterSeverity::${slotSev.OPENAI_FILTER},\n`;
  s += `            Self::AiSlot(AiProvider::Anthropic, _) => FilterSeverity::${slotSev.ANTHROPIC_FILTER},\n`;
  s += `            Self::AiSlot(AiProvider::Gemini, _) => FilterSeverity::${slotSev.GEMINI_FILTER},\n`;
  s += `            Self::AiSlot(AiProvider::Ollama, _) => FilterSeverity::${slotSev.OLLAMA_FILTER},\n`;
  for (const e of fixed) s += `            Self::${e.variant} => FilterSeverity::${e.severity},\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

function emitBitPos() {
  let s = HEADER('`FilterKey::bit_pos`');
  s += `use super::{AiProvider, FilterKey};\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// Internal bit position (kept for on-disk-format parity with the Java port).\n`;
  s += `    pub fn bit_pos(&self) -> i32 {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(n) => ${slotBase.CUSTOM_FILTER} + *n as i32,\n`;
  s += `            Self::CustomExtractorSlot(n) => ${slotBase.CUSTOM_EXTRACTOR} + *n as i32,\n`;
  s += `            Self::CustomJavaScriptSlot(n) => ${slotBase.CUSTOM_JAVASCRIPT} + *n as i32,\n`;
  s += `            Self::AiSlot(AiProvider::Openai, n) => ${slotBase.OPENAI_FILTER} + *n as i32,\n`;
  s += `            Self::AiSlot(AiProvider::Anthropic, n) => ${slotBase.ANTHROPIC_FILTER} + *n as i32,\n`;
  s += `            Self::AiSlot(AiProvider::Gemini, n) => ${slotBase.GEMINI_FILTER} + *n as i32,\n`;
  s += `            Self::AiSlot(AiProvider::Ollama, n) => ${slotBase.OLLAMA_FILTER} + *n as i32,\n`;
  for (const e of fixed) s += `            Self::${e.variant} => ${e.bitPos},\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

function emitWatermark() {
  let s = HEADER('`FilterKey::has_watermark`');
  s += `use super::FilterKey;\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// True when the i18n template needs a threshold substitution (e.g. "over X characters").\n`;
  s += `    /// Slot filters always have a watermark (the slot number).\n`;
  s += `    pub fn has_watermark(&self) -> bool {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(_)\n`;
  s += `            | Self::CustomExtractorSlot(_)\n`;
  s += `            | Self::CustomJavaScriptSlot(_)\n`;
  s += `            | Self::AiSlot(_, _) => true,\n`;
  for (const e of fixed) s += `            Self::${e.variant} => ${e.hasWatermark ? 'true' : 'false'},\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

function emitDeprecated() {
  const deprecated = fixed.filter((e) => e.isDeprecated);
  let s = HEADER('`FilterKey::is_deprecated`');
  s += `use super::FilterKey;\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// True if the Java source lists this key in the DEPRECATED EnumSet.\n`;
  s += `    pub fn is_deprecated(&self) -> bool {\n`;
  if (deprecated.length === 0) {
    s += `        false\n    }\n}\n`;
    return s;
  }
  s += `        matches!(\n            self,\n`;
  s += deprecated.map((e) => `            Self::${e.variant}`).join(' |\n') + '\n';
  s += `        )\n    }\n}\n`;
  return s;
}

function emitDisplay() {
  let s = HEADER('`FilterKey::display_name`');
  s += `use std::borrow::Cow;\n\nuse super::FilterKey;\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// Human-readable label. Slot filters render as e.g. "Custom Search 42".\n`;
  s += `    pub fn display_name(&self) -> Cow<'static, str> {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(n) => Cow::Owned(format!("Custom Search {n}")),\n`;
  s += `            Self::CustomExtractorSlot(n) => Cow::Owned(format!("Custom Extractor {n}")),\n`;
  s += `            Self::CustomJavaScriptSlot(n) => Cow::Owned(format!("Custom JavaScript {n}")),\n`;
  s += `            Self::AiSlot(p, n) => Cow::Owned(format!("{} Prompt {n}", p.display_name())),\n`;
  for (const e of fixed) s += `            Self::${e.variant} => Cow::Borrowed("${e.displayName}"),\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

function emitSerdeKey() {
  let s = HEADER('`FilterKey::serde_key` + `from_serde_key`');
  s += `use std::borrow::Cow;\n\nuse super::{AiProvider, FilterKey};\n\n`;
  s += `impl FilterKey {\n`;
  s += `    /// Canonical snake_case wire/DB key.\n`;
  s += `    pub fn serde_key(&self) -> Cow<'static, str> {\n`;
  s += `        match self {\n`;
  s += `            Self::CustomSearchSlot(n) => Cow::Owned(format!("custom_search_slot_{n}")),\n`;
  s += `            Self::CustomExtractorSlot(n) => Cow::Owned(format!("custom_extractor_slot_{n}")),\n`;
  s += `            Self::CustomJavaScriptSlot(n) => Cow::Owned(format!("custom_javascript_slot_{n}")),\n`;
  s += `            Self::AiSlot(p, n) => Cow::Owned(format!("ai_{}_slot_{n}", p.as_str())),\n`;
  for (const e of fixed) s += `            Self::${e.variant} => Cow::Borrowed("${e.serde}"),\n`;
  s += `        }\n    }\n\n`;
  s += `    /// Parse a serde key back into a FilterKey. Returns \`None\` for unknown strings.\n`;
  s += `    pub fn from_serde_key(s: &str) -> Option<Self> {\n`;
  s += `        if let Some(rest) = s.strip_prefix("custom_search_slot_") {\n`;
  s += `            if let Ok(n) = rest.parse::<u8>() {\n`;
  s += `                if (1..=100).contains(&n) { return Some(Self::CustomSearchSlot(n)); }\n`;
  s += `            }\n        }\n`;
  s += `        if let Some(rest) = s.strip_prefix("custom_extractor_slot_") {\n`;
  s += `            if let Ok(n) = rest.parse::<u8>() {\n`;
  s += `                if (1..=100).contains(&n) { return Some(Self::CustomExtractorSlot(n)); }\n`;
  s += `            }\n        }\n`;
  s += `        if let Some(rest) = s.strip_prefix("custom_javascript_slot_") {\n`;
  s += `            if let Ok(n) = rest.parse::<u8>() {\n`;
  s += `                if (1..=100).contains(&n) { return Some(Self::CustomJavaScriptSlot(n)); }\n`;
  s += `            }\n        }\n`;
  s += `        if let Some(rest) = s.strip_prefix("ai_") {\n`;
  s += `            if let Some(idx) = rest.find("_slot_") {\n`;
  s += `                let provider = &rest[..idx];\n`;
  s += `                let n_str = &rest[idx + "_slot_".len()..];\n`;
  s += `                if let (Some(p), Ok(n)) = (AiProvider::parse(provider), n_str.parse::<u8>()) {\n`;
  s += `                    if (1..=100).contains(&n) { return Some(Self::AiSlot(p, n)); }\n`;
  s += `                }\n            }\n        }\n`;
  s += `        match s {\n`;
  for (const e of fixed) s += `            "${e.serde}" => Some(Self::${e.variant}),\n`;
  s += `            _ => None,\n`;
  s += `        }\n    }\n}\n`;
  return s;
}

const files = {
  'mod.rs':             emitModRs(),
  'tab.rs':             emitTab(),
  'i18n.rs':            emitI18n(),
  'filter_key_type.rs': emitFilterKeyType(),
  'severity.rs':        emitSeverity(),
  'bit_pos.rs':         emitBitPos(),
  'watermark.rs':       emitWatermark(),
  'deprecated.rs':      emitDeprecated(),
  'display.rs':         emitDisplay(),
  'serde_key.rs':       emitSerdeKey(),
};

fs.mkdirSync(OUT_DIR, { recursive: true });
for (const [name, content] of Object.entries(files)) {
  fs.writeFileSync(path.join(OUT_DIR, name), content);
}

process.stderr.write(
  `Generated ${Object.keys(files).length} files in ${OUT_DIR}:\n` +
    `  fixed variants: ${fixed.length}\n` +
    `  slot variants:  4 (collapsed from 700 Java entries)\n` +
    `  deprecated: ${fixed.filter((e) => e.isDeprecated).length}\n` +
    `Per-tab fixed counts:\n` +
    Object.entries(byTab)
      .sort((a, b) => b[1].length - a[1].length)
      .map(([k, v]) => `  ${k.padEnd(20)} ${v.length}`)
      .join('\n') +
    '\n',
);
