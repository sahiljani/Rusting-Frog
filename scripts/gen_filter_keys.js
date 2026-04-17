// Parses SeoElementFilterKey.java and generates a clean, idiomatic Rust
// FilterKey enum. Built-in filters each get a variant; the seven 100-slot
// families (Custom Search, Custom Extractor, Custom JavaScript, and AI
// extractors for OpenAI / Anthropic / Gemini / Ollama — 700 Java entries
// total) collapse into four parameterized variants that hold slot data.
//
// Usage: node scripts/gen_filter_keys.js > crates/core/src/filter_key.rs

const fs = require('fs');

const JAVA = 'C:/Users/User/Desktop/screamingfrog1/sources/seo/spider/seoelements/SeoElementFilterKey.java';
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

// Preserve readable compound words in CamelCase conversion.
const COMPOUND = {
  JAVASCRIPT: 'JavaScript',
  HTML: 'Html',
  CSS: 'Css',
  URL: 'Url',
  URLS: 'Urls',
  HTTP: 'Http',
  HTTPS: 'Https',
  PDF: 'Pdf',
  XML: 'Xml',
  AMP: 'Amp',
  JSON: 'Json',
  RSS: 'Rss',
  API: 'Api',
  GZIP: 'Gzip',
  GA: 'Ga',
  GSC: 'Gsc',
  LCP: 'Lcp',
  FCP: 'Fcp',
  CLS: 'Cls',
  FID: 'Fid',
  TTI: 'Tti',
  TTFB: 'Ttfb',
  INP: 'Inp',
  TBT: 'Tbt',
  SEO: 'Seo',
  AI: 'Ai',
  UUID: 'Uuid',
  ARIA: 'Aria',
  AVIF: 'Avif',
  WEBP: 'Webp',
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

function toSnake(name) {
  return name.toLowerCase();
}

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

// Slot-family prefixes that collapse into parameterized variants.
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

// Renames for the _ALL aggregators so Rust variant names reflect their tab
// rather than the legacy Java prefix.
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

// ---- Emit Rust ----
let out = '';
out += `// AUTO-GENERATED by scripts/gen_filter_keys.js from\n`;
out += `// screamingfrog1/sources/seo/spider/seoelements/SeoElementFilterKey.java\n`;
out += `// Do not edit by hand — regenerate with \`node scripts/gen_filter_keys.js\`.\n`;
out += `//\n`;
out += `// The 700 user-configurable slot filters in the Java source\n`;
out += `// (CUSTOM_FILTER_N, CUSTOM_EXTRACTOR_N, CUSTOM_JAVASCRIPT_N, plus\n`;
out += `// {OPENAI,ANTHROPIC,GEMINI,OLLAMA}_FILTER_N, 100 each) are collapsed\n`;
out += `// into four parameterized Rust variants that hold the slot number.\n`;
out += `// That drops the enum from 1157 variants to ${fixed.length} + 4.\n\n`;
out += `#![allow(clippy::too_many_lines)]\n\n`;
out += `use std::borrow::Cow;\n`;
out += `use std::fmt;\n`;
out += `use serde::{de, Deserialize, Deserializer, Serialize, Serializer};\n\n`;
out += `use crate::tab::TabKey;\n\n`;

out += `/// Which AI provider a user-configurable AI-extraction slot targets.\n`;
out += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n`;
out += `pub enum AiProvider {\n`;
out += `    Openai,\n`;
out += `    Anthropic,\n`;
out += `    Gemini,\n`;
out += `    Ollama,\n`;
out += `}\n\n`;

out += `impl AiProvider {\n`;
out += `    pub const ALL: &'static [Self] = &[Self::Openai, Self::Anthropic, Self::Gemini, Self::Ollama];\n\n`;
out += `    pub fn as_str(&self) -> &'static str {\n`;
out += `        match self {\n`;
out += `            Self::Openai => "openai",\n`;
out += `            Self::Anthropic => "anthropic",\n`;
out += `            Self::Gemini => "gemini",\n`;
out += `            Self::Ollama => "ollama",\n`;
out += `        }\n`;
out += `    }\n\n`;
out += `    pub fn display_name(&self) -> &'static str {\n`;
out += `        match self {\n`;
out += `            Self::Openai => "OpenAI",\n`;
out += `            Self::Anthropic => "Anthropic",\n`;
out += `            Self::Gemini => "Gemini",\n`;
out += `            Self::Ollama => "Ollama",\n`;
out += `        }\n`;
out += `    }\n\n`;
out += `    pub fn parse(s: &str) -> Option<Self> {\n`;
out += `        match s {\n`;
out += `            "openai" => Some(Self::Openai),\n`;
out += `            "anthropic" => Some(Self::Anthropic),\n`;
out += `            "gemini" => Some(Self::Gemini),\n`;
out += `            "ollama" => Some(Self::Ollama),\n`;
out += `            _ => None,\n`;
out += `        }\n`;
out += `    }\n`;
out += `}\n\n`;

out += `/// Classification of how a filter is evaluated.\n`;
out += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n`;
out += `#[serde(rename_all = "snake_case")]\n`;
out += `pub enum FilterKeyType {\n`;
out += `    Normal,\n`;
out += `    PostCrawlAnalysis,\n`;
out += `    SpellingAndGrammar,\n`;
out += `}\n\n`;

out += `/// STAT = informational counter; ISSUE = flagged problem.\n`;
out += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n`;
out += `#[serde(rename_all = "snake_case")]\n`;
out += `pub enum FilterSeverity {\n`;
out += `    Stat,\n`;
out += `    Issue,\n`;
out += `}\n\n`;

out += `/// Semantic classification returned by [\`FilterKey::kind\`].\n`;
out += `#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n`;
out += `pub enum FilterKind {\n`;
out += `    /// A fixed filter baked into the crawler (e.g. \`TitleMissing\`).\n`;
out += `    BuiltIn,\n`;
out += `    /// User-defined Custom Search slot, 1..=100.\n`;
out += `    CustomSearchSlot(u8),\n`;
out += `    /// User-defined Custom Extractor slot, 1..=100.\n`;
out += `    CustomExtractorSlot(u8),\n`;
out += `    /// User-defined Custom JavaScript slot, 1..=100.\n`;
out += `    CustomJavaScriptSlot(u8),\n`;
out += `    /// User-defined AI-extraction slot, 1..=100, per provider.\n`;
out += `    AiSlot(AiProvider, u8),\n`;
out += `}\n\n`;

const TAB_ORDER = [
  'INTERNAL', 'EXTERNAL', 'PAGE_TITLES', 'META_DESCRIPTION', 'META_KEYWORDS',
  'H1', 'H2', 'IMAGES', 'CANONICALS', 'PAGINATION', 'DIRECTIVES', 'HREFLANG',
  'JAVASCRIPT', 'AMP', 'LINKS', 'RESPONSE_CODE', 'URL', 'CONTENT', 'SECURITY',
  'SITEMAPS', 'STRUCTURED_DATA', 'MOBILE', 'VALIDATION', 'PAGE_SPEED',
  'ANALYTICS', 'SEARCH_CONSOLE', 'LINK_METRICS', 'PARITY',
  'CUSTOM_SEARCH', 'CUSTOM_EXTRACTION', 'CUSTOM_JAVASCRIPT', 'AI',
  'ACCESSIBILITY', 'UNDEF',
];

out += `/// Every filter shown in the SF-style UI, across all 33 tabs.\n`;
out += `///\n`;
out += `/// Ported from \`seo.spider.seoelements.SeoElementFilterKey\`. Built-in\n`;
out += `/// filters each get one variant; the four user-configurable slot families\n`;
out += `/// collapse into parameterized variants — see [\`FilterKind\`].\n`;
out += `///\n`;
out += `/// Serde format: snake_case. Slot variants serialize as\n`;
out += `/// \`custom_search_slot_<n>\`, \`custom_extractor_slot_<n>\`,\n`;
out += `/// \`custom_javascript_slot_<n>\`, or \`ai_<provider>_slot_<n>\`.\n`;
out += `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n`;
out += `pub enum FilterKey {\n`;

for (const tabKey of TAB_ORDER) {
  const group = byTab[tabKey];
  if (!group || !group.length) continue;
  out += `    // ---- ${TABS[tabKey]} (${group.length} built-in) ----\n`;
  for (const e of group) out += `    ${e.variant},\n`;
  out += `\n`;
}

out += `    // ---- Parameterized slot filters (1..=100 each) ----\n`;
out += `    /// User-defined Custom Search slot filter. (Java: CUSTOM_FILTER_N)\n`;
out += `    CustomSearchSlot(u8),\n`;
out += `    /// User-defined Custom Extractor slot filter. (Java: CUSTOM_EXTRACTOR_N)\n`;
out += `    CustomExtractorSlot(u8),\n`;
out += `    /// User-defined Custom JavaScript slot filter. (Java: CUSTOM_JAVASCRIPT_N)\n`;
out += `    CustomJavaScriptSlot(u8),\n`;
out += `    /// User-defined AI-extraction slot, per provider.\n`;
out += `    /// (Java: {OPENAI,ANTHROPIC,GEMINI,OLLAMA}_FILTER_N)\n`;
out += `    AiSlot(AiProvider, u8),\n`;
out += `}\n\n`;

// impl FilterKey
out += `impl FilterKey {\n`;

out += `    /// Semantic grouping of this filter.\n`;
out += `    pub fn kind(&self) -> FilterKind {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(n) => FilterKind::CustomSearchSlot(*n),\n`;
out += `            Self::CustomExtractorSlot(n) => FilterKind::CustomExtractorSlot(*n),\n`;
out += `            Self::CustomJavaScriptSlot(n) => FilterKind::CustomJavaScriptSlot(*n),\n`;
out += `            Self::AiSlot(p, n) => FilterKind::AiSlot(*p, *n),\n`;
out += `            _ => FilterKind::BuiltIn,\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// Which UI tab this filter belongs to.\n`;
out += `    pub fn tab(&self) -> TabKey {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(_) => TabKey::CustomSearch,\n`;
out += `            Self::CustomExtractorSlot(_) => TabKey::CustomExtraction,\n`;
out += `            Self::CustomJavaScriptSlot(_) => TabKey::CustomJavaScript,\n`;
out += `            Self::AiSlot(_, _) => TabKey::Ai,\n`;
for (const e of fixed) out += `            Self::${e.variant} => TabKey::${e.tabRust},\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// i18n key for the filter's display name (e.g. "tab.internal.filter.html").\n`;
out += `    pub fn i18n_key(&self) -> &'static str {\n`;
out += `        match self {\n`;
for (const f of SLOT_FAMILIES) {
  if (f.variant === 'AiSlot') {
    out += `            Self::AiSlot(AiProvider::${f.provider}, _) => "${f.i18n}",\n`;
  } else {
    out += `            Self::${f.variant}(_) => "${f.i18n}",\n`;
  }
}
for (const e of fixed) out += `            Self::${e.variant} => "${e.i18n}",\n`;
out += `        }\n`;
out += `    }\n\n`;

const slotFkt = {};
const slotSev = {};
for (const f of SLOT_FAMILIES) {
  slotFkt[f.prefix] = FILTER_KEY_TYPES[slots[f.prefix][0].fkt];
  slotSev[f.prefix] = slots[f.prefix][0].sev === 'ISSUE' ? 'Issue' : 'Stat';
}

out += `    /// Evaluation classification: normal / post-crawl / spelling.\n`;
out += `    pub fn filter_key_type(&self) -> FilterKeyType {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(_) => FilterKeyType::${slotFkt.CUSTOM_FILTER},\n`;
out += `            Self::CustomExtractorSlot(_) => FilterKeyType::${slotFkt.CUSTOM_EXTRACTOR},\n`;
out += `            Self::CustomJavaScriptSlot(_) => FilterKeyType::${slotFkt.CUSTOM_JAVASCRIPT},\n`;
out += `            Self::AiSlot(AiProvider::Openai, _) => FilterKeyType::${slotFkt.OPENAI_FILTER},\n`;
out += `            Self::AiSlot(AiProvider::Anthropic, _) => FilterKeyType::${slotFkt.ANTHROPIC_FILTER},\n`;
out += `            Self::AiSlot(AiProvider::Gemini, _) => FilterKeyType::${slotFkt.GEMINI_FILTER},\n`;
out += `            Self::AiSlot(AiProvider::Ollama, _) => FilterKeyType::${slotFkt.OLLAMA_FILTER},\n`;
for (const e of fixed) out += `            Self::${e.variant} => FilterKeyType::${e.fktRust},\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// STAT = counter, ISSUE = flagged problem.\n`;
out += `    pub fn severity(&self) -> FilterSeverity {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(_) => FilterSeverity::${slotSev.CUSTOM_FILTER},\n`;
out += `            Self::CustomExtractorSlot(_) => FilterSeverity::${slotSev.CUSTOM_EXTRACTOR},\n`;
out += `            Self::CustomJavaScriptSlot(_) => FilterSeverity::${slotSev.CUSTOM_JAVASCRIPT},\n`;
out += `            Self::AiSlot(AiProvider::Openai, _) => FilterSeverity::${slotSev.OPENAI_FILTER},\n`;
out += `            Self::AiSlot(AiProvider::Anthropic, _) => FilterSeverity::${slotSev.ANTHROPIC_FILTER},\n`;
out += `            Self::AiSlot(AiProvider::Gemini, _) => FilterSeverity::${slotSev.GEMINI_FILTER},\n`;
out += `            Self::AiSlot(AiProvider::Ollama, _) => FilterSeverity::${slotSev.OLLAMA_FILTER},\n`;
for (const e of fixed) out += `            Self::${e.variant} => FilterSeverity::${e.severity},\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// Internal bit position (kept for on-disk-format parity with the Java port).\n`;
out += `    pub fn bit_pos(&self) -> i32 {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(n) => ${slotBase.CUSTOM_FILTER} + *n as i32,\n`;
out += `            Self::CustomExtractorSlot(n) => ${slotBase.CUSTOM_EXTRACTOR} + *n as i32,\n`;
out += `            Self::CustomJavaScriptSlot(n) => ${slotBase.CUSTOM_JAVASCRIPT} + *n as i32,\n`;
out += `            Self::AiSlot(AiProvider::Openai, n) => ${slotBase.OPENAI_FILTER} + *n as i32,\n`;
out += `            Self::AiSlot(AiProvider::Anthropic, n) => ${slotBase.ANTHROPIC_FILTER} + *n as i32,\n`;
out += `            Self::AiSlot(AiProvider::Gemini, n) => ${slotBase.GEMINI_FILTER} + *n as i32,\n`;
out += `            Self::AiSlot(AiProvider::Ollama, n) => ${slotBase.OLLAMA_FILTER} + *n as i32,\n`;
for (const e of fixed) out += `            Self::${e.variant} => ${e.bitPos},\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// True when the i18n template needs a threshold substitution (e.g. "over X characters").\n`;
out += `    /// Slot filters always have a watermark (the slot number).\n`;
out += `    pub fn has_watermark(&self) -> bool {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(_)\n`;
out += `            | Self::CustomExtractorSlot(_)\n`;
out += `            | Self::CustomJavaScriptSlot(_)\n`;
out += `            | Self::AiSlot(_, _) => true,\n`;
for (const e of fixed) out += `            Self::${e.variant} => ${e.hasWatermark ? 'true' : 'false'},\n`;
out += `        }\n`;
out += `    }\n\n`;

const deprecated = fixed.filter((e) => e.isDeprecated);
out += `    /// True if the Java source lists this key in the DEPRECATED EnumSet.\n`;
out += `    pub fn is_deprecated(&self) -> bool {\n`;
if (deprecated.length === 0) {
  out += `        false\n`;
} else {
  out += `        matches!(\n`;
  out += `            self,\n`;
  out += deprecated.map((e) => `            Self::${e.variant}`).join(' |\n') + '\n';
  out += `        )\n`;
}
out += `    }\n\n`;

out += `    /// Human-readable label. Slot filters render as e.g. "Custom Search 42".\n`;
out += `    pub fn display_name(&self) -> Cow<'static, str> {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(n) => Cow::Owned(format!("Custom Search {n}")),\n`;
out += `            Self::CustomExtractorSlot(n) => Cow::Owned(format!("Custom Extractor {n}")),\n`;
out += `            Self::CustomJavaScriptSlot(n) => Cow::Owned(format!("Custom JavaScript {n}")),\n`;
out += `            Self::AiSlot(p, n) => Cow::Owned(format!("{} Prompt {n}", p.display_name())),\n`;
for (const e of fixed) out += `            Self::${e.variant} => Cow::Borrowed("${e.displayName}"),\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// Canonical snake_case wire/DB key.\n`;
out += `    pub fn serde_key(&self) -> Cow<'static, str> {\n`;
out += `        match self {\n`;
out += `            Self::CustomSearchSlot(n) => Cow::Owned(format!("custom_search_slot_{n}")),\n`;
out += `            Self::CustomExtractorSlot(n) => Cow::Owned(format!("custom_extractor_slot_{n}")),\n`;
out += `            Self::CustomJavaScriptSlot(n) => Cow::Owned(format!("custom_javascript_slot_{n}")),\n`;
out += `            Self::AiSlot(p, n) => Cow::Owned(format!("ai_{}_slot_{n}", p.as_str())),\n`;
for (const e of fixed) out += `            Self::${e.variant} => Cow::Borrowed("${e.serde}"),\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// Parse a serde key back into a FilterKey. Returns \`None\` for unknown strings.\n`;
out += `    pub fn from_serde_key(s: &str) -> Option<Self> {\n`;
out += `        if let Some(rest) = s.strip_prefix("custom_search_slot_") {\n`;
out += `            if let Ok(n) = rest.parse::<u8>() {\n`;
out += `                if (1..=100).contains(&n) { return Some(Self::CustomSearchSlot(n)); }\n`;
out += `            }\n`;
out += `        }\n`;
out += `        if let Some(rest) = s.strip_prefix("custom_extractor_slot_") {\n`;
out += `            if let Ok(n) = rest.parse::<u8>() {\n`;
out += `                if (1..=100).contains(&n) { return Some(Self::CustomExtractorSlot(n)); }\n`;
out += `            }\n`;
out += `        }\n`;
out += `        if let Some(rest) = s.strip_prefix("custom_javascript_slot_") {\n`;
out += `            if let Ok(n) = rest.parse::<u8>() {\n`;
out += `                if (1..=100).contains(&n) { return Some(Self::CustomJavaScriptSlot(n)); }\n`;
out += `            }\n`;
out += `        }\n`;
out += `        if let Some(rest) = s.strip_prefix("ai_") {\n`;
out += `            if let Some(idx) = rest.find("_slot_") {\n`;
out += `                let provider = &rest[..idx];\n`;
out += `                let n_str = &rest[idx + "_slot_".len()..];\n`;
out += `                if let (Some(p), Ok(n)) = (AiProvider::parse(provider), n_str.parse::<u8>()) {\n`;
out += `                    if (1..=100).contains(&n) { return Some(Self::AiSlot(p, n)); }\n`;
out += `                }\n`;
out += `            }\n`;
out += `        }\n`;
out += `        match s {\n`;
for (const e of fixed) out += `            "${e.serde}" => Some(Self::${e.variant}),\n`;
out += `            _ => None,\n`;
out += `        }\n`;
out += `    }\n\n`;

out += `    /// Every filter key, in a stable order (all fixed variants first, then slot families).\n`;
out += `    pub fn all() -> Vec<FilterKey> {\n`;
out += `        let mut v = Vec::with_capacity(${fixed.length} + 700);\n`;
out += `        v.extend_from_slice(&[\n`;
for (const e of fixed) out += `            Self::${e.variant},\n`;
out += `        ]);\n`;
out += `        for n in 1..=100u8 { v.push(Self::CustomSearchSlot(n)); }\n`;
out += `        for n in 1..=100u8 { v.push(Self::CustomExtractorSlot(n)); }\n`;
out += `        for n in 1..=100u8 { v.push(Self::CustomJavaScriptSlot(n)); }\n`;
out += `        for p in AiProvider::ALL.iter().copied() {\n`;
out += `            for n in 1..=100u8 { v.push(Self::AiSlot(p, n)); }\n`;
out += `        }\n`;
out += `        v\n`;
out += `    }\n\n`;

out += `    /// All filter keys for a given tab, in declaration order.\n`;
out += `    pub fn for_tab(tab: TabKey) -> Vec<FilterKey> {\n`;
out += `        Self::all().into_iter().filter(|k| k.tab() == tab).collect()\n`;
out += `    }\n`;
out += `}\n\n`;

out += `// ---- Serde ----\n\n`;
out += `impl Serialize for FilterKey {\n`;
out += `    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {\n`;
out += `        s.serialize_str(&self.serde_key())\n`;
out += `    }\n`;
out += `}\n\n`;
out += `impl<'de> Deserialize<'de> for FilterKey {\n`;
out += `    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {\n`;
out += `        struct V;\n`;
out += `        impl<'de> de::Visitor<'de> for V {\n`;
out += `            type Value = FilterKey;\n`;
out += `            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {\n`;
out += `                f.write_str("a filter_key string")\n`;
out += `            }\n`;
out += `            fn visit_str<E: de::Error>(self, s: &str) -> Result<FilterKey, E> {\n`;
out += `                FilterKey::from_serde_key(s)\n`;
out += `                    .ok_or_else(|| E::custom(format!("unknown filter_key: {s}")))\n`;
out += `            }\n`;
out += `        }\n`;
out += `        d.deserialize_str(V)\n`;
out += `    }\n`;
out += `}\n`;

process.stdout.write(out);

process.stderr.write(
  `Generated FilterKey:\n` +
    `  fixed variants: ${fixed.length}\n` +
    `  slot variants:  4 (collapsed from 700 Java entries)\n` +
    `  deprecated: ${deprecated.length}\n` +
    `Per-tab fixed counts:\n` +
    Object.entries(byTab)
      .sort((a, b) => b[1].length - a[1].length)
      .map(([k, v]) => `  ${k.padEnd(20)} ${v.length}`)
      .join('\n') +
    '\n',
);
