//! Error categories: a fixed two-level list (area → category) the model must choose from.
//! Top level shared by all languages (modeled on MERLIN's groups); subcategories map to ERRANT
//! (English) and MERLIN (German). Rationale and sources: `docs/research/stats-system.md` §3.4, §4.2.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Top level, shared by all languages so stats compare across them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ErrorArea {
    Grammar,
    Lexis,
    Orthography,
    Register,
    Discourse,
    Other,
}

/// Ferris's treatable/untreatable distinction: rule errors get a rule in reminders, item errors
/// get the specific item to remember (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Rule,
    Item,
}

macro_rules! categories {
    ($( $variant:ident = $id:literal, $area:ident, $kind:expr, $langs:expr, $label:literal, $desc:literal; )*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
        #[ts(export)]
        pub enum ErrorCategory {
            $( #[serde(rename = $id)] $variant, )*
        }

        impl ErrorCategory {
            pub const ALL: &'static [ErrorCategory] = &[$( ErrorCategory::$variant, )*];

            pub fn id(self) -> &'static str {
                match self { $( ErrorCategory::$variant => $id, )* }
            }

            pub fn area(self) -> ErrorArea {
                match self { $( ErrorCategory::$variant => ErrorArea::$area, )* }
            }

            pub fn kind(self) -> Option<ErrorKind> {
                match self { $( ErrorCategory::$variant => $kind, )* }
            }

            /// Languages the category applies to; empty means all.
            pub fn languages(self) -> &'static [&'static str] {
                match self { $( ErrorCategory::$variant => $langs, )* }
            }

            pub fn label(self) -> &'static str {
                match self { $( ErrorCategory::$variant => $label, )* }
            }

            /// One line for the model: what belongs in this category.
            pub fn description(self) -> &'static str {
                match self { $( ErrorCategory::$variant => $desc, )* }
            }
        }
    };
}

const ALL_LANGS: &[&str] = &[];
const EN: &[&str] = &["en"];
const DE: &[&str] = &["de"];
const RULE: Option<ErrorKind> = Some(ErrorKind::Rule);
const ITEM: Option<ErrorKind> = Some(ErrorKind::Item);

categories! {
    Article = "grammar.article", Grammar, RULE, ALL_LANGS, "Articles", "a/an/the/zero article; der/ein/kein presence and choice";
    Preposition = "grammar.preposition", Grammar, ITEM, ALL_LANGS, "Prepositions", "a single wrong, missing or extra preposition, incl. prepositional verbs";
    VerbTense = "grammar.verb_tense", Grammar, RULE, ALL_LANGS, "Tense", "wrong tense, aspect, mood or modal choice for the meaning (went vs have gone; Perfekt vs Präteritum; Konjunktiv II)";
    VerbForm = "grammar.verb_form", Grammar, RULE, ALL_LANGS, "Verb forms", "right tense, wrongly built: conjugation, irregular forms, participles, infinitive vs gerund, haben/sein auxiliary";
    Agreement = "grammar.agreement", Grammar, RULE, ALL_LANGS, "Agreement", "subject–verb agreement";
    NounNumber = "grammar.noun_number", Grammar, RULE, ALL_LANGS, "Plurals & countability", "singular/plural, plural formation, countable vs uncountable (informations)";
    Pronoun = "grammar.pronoun", Grammar, RULE, ALL_LANGS, "Pronouns", "pronoun form, reflexives, relative pronouns (in German, a pronoun in the wrong case is grammar.case)";
    AdjAdv = "grammar.adj_adv", Grammar, RULE, EN, "Adjectives & adverbs", "comparative/superlative forms, adjective vs adverb (quick vs quickly)";
    WordOrder = "grammar.word_order", Grammar, RULE, ALL_LANGS, "Word order", "constituent order: German V2 and verb-final, separable prefixes; English question order, adverb placement";
    SentenceStructure = "grammar.sentence_structure", Grammar, RULE, ALL_LANGS, "Sentence structure", "fragments, run-ons, missing subject or finite verb, broken conditionals or clause linkage";
    Case = "grammar.case", Grammar, RULE, DE, "Case", "wrong case of an article, pronoun or noun for its role or preposition (mit der Mann → mit dem Mann)";
    Gender = "grammar.gender", Grammar, ITEM, DE, "Noun gender", "wrong noun gender: the form would be wrong in every case for this noun (das Tisch → der Tisch)";
    AdjectiveEnding = "grammar.adjective_ending", Grammar, RULE, DE, "Adjective endings", "adjective declension endings";
    Valency = "grammar.valency", Grammar, ITEM, DE, "Verb complements", "missing or extra object or reflexive pronoun for the verb";
    Negation = "grammar.negation", Grammar, RULE, DE, "Negation", "nicht vs kein, position of nicht";
    GrammarOther = "grammar.other", Grammar, None, ALL_LANGS, "Other grammar", "a grammar error that fits no other grammar category";

    WordChoice = "lexis.word_choice", Lexis, ITEM, ALL_LANGS, "Word choice", "the word itself has the wrong meaning here, incl. false friends";
    Collocation = "lexis.collocation", Lexis, ITEM, ALL_LANGS, "Collocations", "each word is fine but they don't go together, or a fixed phrase is broken (make a photo; eine Entscheidung machen)";
    WordForm = "lexis.word_form", Lexis, RULE, ALL_LANGS, "Word formation", "wrong derived form or compound (success vs successful)";

    Spelling = "orthography.spelling", Orthography, ITEM, ALL_LANGS, "Spelling", "a misspelled word, incl. umlauts and ß";
    Capitalization = "orthography.capitalization", Orthography, RULE, ALL_LANGS, "Capitalization", "German noun capitalization, Sie/Ihnen, English I, proper names";
    Punctuation = "orthography.punctuation", Orthography, RULE, ALL_LANGS, "Punctuation", "punctuation that is clearly wrong or changes the reading (required German comma before a subordinate clause)";
    WordBoundary = "orthography.word_boundary", Orthography, RULE, ALL_LANGS, "Word boundaries", "joined vs separate vs hyphenated (alot; everyday vs every day)";

    Formality = "register.formality", Register, RULE, ALL_LANGS, "Formality", "too informal or too formal for the chosen tone, incl. shorthand where it doesn't fit";
    Address = "register.address", Register, RULE, DE, "du / Sie", "mixed or wrong form of address, wrong greeting or closing for the tone";

    Connector = "discourse.connector", Discourse, RULE, ALL_LANGS, "Connectors", "a linking word that changes the logic (therefore vs however; deshalb vs trotzdem)";
    Reference = "discourse.reference", Discourse, RULE, EN, "Reference", "unclear or wrong reference to something mentioned earlier";

    Other = "other", Other, None, ALL_LANGS, "Other", "an error that fits nowhere else";
}

impl ErrorCategory {
    pub fn parse(id: &str) -> Option<ErrorCategory> {
        ErrorCategory::ALL.iter().copied().find(|c| c.id() == id)
    }

    pub fn applies_to(self, language: &str) -> bool {
        let langs = self.languages();
        langs.is_empty() || langs.contains(&language)
    }

    /// Repair for an unknown id (§3.9): `grammar.other` if the model at least named grammar,
    /// otherwise the top-level `other`. Never stored as free text.
    pub fn fallback(id: &str) -> ErrorCategory {
        if id.starts_with("grammar") { ErrorCategory::GrammarOther } else { ErrorCategory::Other }
    }

    /// Categories offered to the model for a set of languages.
    pub fn for_languages(languages: &[&str]) -> Vec<ErrorCategory> {
        ErrorCategory::ALL.iter().copied().filter(|c| languages.iter().any(|l| c.applies_to(l))).collect()
    }
}

/// Category metadata for the UI, so the frontend doesn't hard-code the list.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub id: ErrorCategory,
    pub area: ErrorArea,
    pub kind: Option<ErrorKind>,
    pub label: String,
    pub description: String,
    pub languages: Vec<String>,
}

pub fn catalog() -> Vec<CategoryInfo> {
    ErrorCategory::ALL
        .iter()
        .map(|c| CategoryInfo {
            id: *c,
            area: c.area(),
            kind: c.kind(),
            label: c.label().into(),
            description: c.description().into(),
            languages: c.languages().iter().map(|s| s.to_string()).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_prefixed_by_area() {
        let mut seen = std::collections::HashSet::new();
        for c in ErrorCategory::ALL {
            assert!(seen.insert(c.id()), "duplicate id {}", c.id());
            let area = serde_json::to_value(c.area()).unwrap();
            assert!(c.id().split('.').next() == area.as_str(), "{}", c.id());
            assert_eq!(serde_json::to_value(c).unwrap(), c.id());
        }
    }

    #[test]
    fn subcategory_counts_match_the_research() {
        // §4.2: 21 English and 25 German subcategories, plus the top-level `other`.
        let count = |l: &str| ErrorCategory::for_languages(&[l]).iter().filter(|c| **c != ErrorCategory::Other).count();
        assert_eq!(count("en"), 21);
        assert_eq!(count("de"), 25);
    }

    #[test]
    fn unknown_ids_fall_back() {
        assert_eq!(ErrorCategory::fallback("grammar.mood"), ErrorCategory::GrammarOther);
        assert_eq!(ErrorCategory::fallback("style.flow"), ErrorCategory::Other);
    }
}
