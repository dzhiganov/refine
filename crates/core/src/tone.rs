use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Formality {
    Casual,
    Neutral,
    Formal,
}

/// Whether abbreviations, contractions and chat shorthand (pls, thx, imo, gonna) are fine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ShortForms {
    Welcome,
    Sparingly,
    Avoid,
}

/// Context for the model about where the text will be posted. Built-ins can be edited too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Tone {
    pub id: String,
    pub name: String,
    /// Free text: audience, medium, what's normal there.
    pub description: String,
    pub formality: Formality,
    pub short_forms: ShortForms,
    pub builtin: bool,
}

pub const DEFAULT_TONE_ID: &str = "neutral";

pub fn builtin_tones() -> Vec<Tone> {
    let t = |id: &str, name: &str, description: &str, formality, short_forms| Tone {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        formality,
        short_forms,
        builtin: true,
    };
    vec![
        t(
            DEFAULT_TONE_ID,
            "Neutral",
            "General writing for no particular audience.",
            Formality::Neutral,
            ShortForms::Sparingly,
        ),
        t(
            "slack",
            "Slack",
            "Workplace chat with colleagues. Short messages, fragments, lowercase sentence starts, \
             emoji and common chat abbreviations (pls, thx, btw, imo, afaik, lmk) are normal. \
             It should read like a chat message, not an email.",
            Formality::Casual,
            ShortForms::Welcome,
        ),
        t(
            "github",
            "GitHub",
            "Comments on issues, pull requests and discussions. Concise, technical, direct but \
             polite. Markdown, code identifiers and developer shorthand (PR, LGTM, WIP, IMO, nit:, \
             repro) are normal; other chat abbreviations are not.",
            Formality::Neutral,
            ShortForms::Sparingly,
        ),
        t(
            "email",
            "Formal email",
            "Professional email to a client, a manager or someone the writer doesn't know well. \
             Full sentences, no chat abbreviations, contractions only sparingly. In German, use Sie.",
            Formality::Formal,
            ShortForms::Avoid,
        ),
        t(
            "friends",
            "Friends",
            "Personal messages to friends. Anything natural in speech is fine: slang, \
             abbreviations, missing punctuation. In German, use du.",
            Formality::Casual,
            ShortForms::Welcome,
        ),
    ]
}
