//! Integration tests for `FilterKey`.
//!
//! These verify the invariants the generator and the hand-written helpers
//! are supposed to uphold: every filter has a real tab, serde keys are
//! unique, round-trip works, and the slot families expand to exactly 100
//! entries per slot and 400 entries for AI (4 providers × 100).

use std::collections::HashSet;

use sf_core::filter_key::{AiProvider, FilterKey, FilterKind, FilterSeverity};
use sf_core::tab::TabKey;

#[test]
fn every_filter_except_unknown_has_a_real_tab() {
    // `FilterKey::Unknown` is the single sentinel that intentionally maps
    // to `TabKey::Undef`. Everything else must belong to a real tab.
    for f in FilterKey::all() {
        if matches!(f, FilterKey::Unknown) {
            continue;
        }
        let tab = f.tab();
        assert_ne!(
            tab,
            TabKey::Undef,
            "{f:?} maps to TabKey::Undef — every filter must belong to a real tab"
        );
    }
}

#[test]
fn serde_keys_are_unique() {
    let mut seen: HashSet<String> = HashSet::new();
    for f in FilterKey::all() {
        let key = f.serde_key();
        assert!(
            seen.insert(key.to_string()),
            "duplicate serde_key: {key} for {f:?}"
        );
    }
}

#[test]
fn serde_keys_round_trip() {
    for f in FilterKey::all() {
        let key = f.serde_key();
        let round = FilterKey::from_serde_key(&key)
            .unwrap_or_else(|| panic!("could not parse back key {key} from {f:?}"));
        assert_eq!(round, f, "round trip mismatch for {key}");
    }
}

#[test]
fn serde_json_round_trip() {
    // A handful of representative filters, including all 4 slot families.
    let cases = [
        FilterKey::InternalHtml,
        FilterKey::TitleMissing,
        FilterKey::CustomSearchSlot(1),
        FilterKey::CustomSearchSlot(100),
        FilterKey::CustomExtractorSlot(42),
        FilterKey::CustomJavaScriptSlot(7),
        FilterKey::AiSlot(AiProvider::Openai, 3),
        FilterKey::AiSlot(AiProvider::Anthropic, 99),
        FilterKey::AiSlot(AiProvider::Gemini, 1),
        FilterKey::AiSlot(AiProvider::Ollama, 100),
    ];
    for f in cases {
        let json = serde_json::to_string(&f).expect("serialize");
        let back: FilterKey = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(f, back, "json round-trip failed for {f:?} (json={json})");
    }
}

#[test]
fn slot_serde_shape() {
    assert_eq!(
        serde_json::to_string(&FilterKey::CustomSearchSlot(42)).unwrap(),
        "\"custom_search_slot_42\""
    );
    assert_eq!(
        serde_json::to_string(&FilterKey::CustomExtractorSlot(7)).unwrap(),
        "\"custom_extractor_slot_7\""
    );
    assert_eq!(
        serde_json::to_string(&FilterKey::CustomJavaScriptSlot(1)).unwrap(),
        "\"custom_javascript_slot_1\""
    );
    assert_eq!(
        serde_json::to_string(&FilterKey::AiSlot(AiProvider::Openai, 42)).unwrap(),
        "\"ai_openai_slot_42\""
    );
    assert_eq!(
        serde_json::to_string(&FilterKey::AiSlot(AiProvider::Anthropic, 1)).unwrap(),
        "\"ai_anthropic_slot_1\""
    );
}

#[test]
fn slot_tab_mapping_is_correct() {
    for n in 1..=100u8 {
        assert_eq!(FilterKey::CustomSearchSlot(n).tab(), TabKey::CustomSearch);
        assert_eq!(
            FilterKey::CustomExtractorSlot(n).tab(),
            TabKey::CustomExtraction
        );
        assert_eq!(
            FilterKey::CustomJavaScriptSlot(n).tab(),
            TabKey::CustomJavaScript
        );
        for p in AiProvider::ALL {
            assert_eq!(FilterKey::AiSlot(*p, n).tab(), TabKey::Ai);
        }
    }
}

#[test]
fn for_tab_returns_expected_slot_counts() {
    // 1 "_ALL" aggregator + 100 numbered slots.
    assert_eq!(FilterKey::for_tab(TabKey::CustomSearch).len(), 101);
    assert_eq!(FilterKey::for_tab(TabKey::CustomExtraction).len(), 101);
    assert_eq!(FilterKey::for_tab(TabKey::CustomJavaScript).len(), 101);
    // AI: 1 aggregator + 4 providers × 100 = 401.
    assert_eq!(FilterKey::for_tab(TabKey::Ai).len(), 401);
}

#[test]
fn for_tab_is_non_empty_for_every_real_tab() {
    for tab in TabKey::all() {
        if matches!(tab, TabKey::Undef) {
            continue;
        }
        let filters = FilterKey::for_tab(*tab);
        assert!(
            !filters.is_empty(),
            "{tab:?} has no filters — every real tab must expose at least one"
        );
    }
}

#[test]
fn unknown_serde_key_returns_none() {
    assert!(FilterKey::from_serde_key("not_a_real_filter").is_none());
    // Slot index 0 and > 100 are invalid.
    assert!(FilterKey::from_serde_key("custom_search_slot_0").is_none());
    assert!(FilterKey::from_serde_key("custom_search_slot_101").is_none());
    assert!(FilterKey::from_serde_key("ai_openai_slot_0").is_none());
    assert!(FilterKey::from_serde_key("ai_openai_slot_101").is_none());
    // Unknown provider.
    assert!(FilterKey::from_serde_key("ai_grok_slot_1").is_none());
}

#[test]
fn unknown_json_key_fails_to_deserialize() {
    let r: Result<FilterKey, _> = serde_json::from_str("\"not_real\"");
    assert!(r.is_err(), "deserializing unknown key must fail");
}

#[test]
fn kind_classifies_slots_vs_built_in() {
    assert_eq!(FilterKey::InternalHtml.kind(), FilterKind::BuiltIn);
    assert_eq!(
        FilterKey::CustomSearchSlot(42).kind(),
        FilterKind::CustomSearchSlot(42)
    );
    assert_eq!(
        FilterKey::AiSlot(AiProvider::Openai, 7).kind(),
        FilterKind::AiSlot(AiProvider::Openai, 7)
    );
}

#[test]
fn slot_display_names_include_the_number() {
    assert_eq!(
        FilterKey::CustomSearchSlot(42).display_name(),
        "Custom Search 42"
    );
    assert_eq!(
        FilterKey::CustomExtractorSlot(7).display_name(),
        "Custom Extractor 7"
    );
    assert_eq!(
        FilterKey::CustomJavaScriptSlot(1).display_name(),
        "Custom JavaScript 1"
    );
    assert_eq!(
        FilterKey::AiSlot(AiProvider::Openai, 3).display_name(),
        "OpenAI Prompt 3"
    );
}

#[test]
fn severity_is_consistent_within_slot_family() {
    let first_sev = FilterKey::CustomSearchSlot(1).severity();
    for n in 1..=100u8 {
        assert_eq!(
            FilterKey::CustomSearchSlot(n).severity(),
            first_sev,
            "CustomSearchSlot({n}) has a different severity from slot 1"
        );
    }
    // Sanity: at least one built-in is an issue and at least one is a stat.
    let all = FilterKey::all();
    assert!(all.iter().any(|f| f.severity() == FilterSeverity::Issue));
    assert!(all.iter().any(|f| f.severity() == FilterSeverity::Stat));
}
