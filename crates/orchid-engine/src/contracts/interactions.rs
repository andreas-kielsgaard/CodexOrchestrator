use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Provider-neutral answer to a pending runtime request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeInteractionResponse {
    Choose { choice_id: String },
    Answer { answers: BTreeMap<String, Vec<String>> },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRequestKind {
    /// Choose one of the offered choices to allow or decline an action.
    Approval,
    /// Answer every question.
    Questions,
    /// Complete an action outside Orchid, then report it with one of the choices.
    ExternalAction,
    /// Orchid cannot answer this request; the provider has already declined it.
    #[default]
    Unsupported,
}

/// A provider request awaiting the user, in Orchid's display shape. Every provider produces this
/// shape; the provider keeps the mapping from choice and question IDs to its native response.
/// Field names are the ones already stored in event history, so older records decode unchanged.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRequest {
    pub id: String,
    #[serde(default)]
    pub kind: RuntimeRequestKind,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Provider-described permissions, shown to the user as offered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_root: Option<String>,
    #[serde(default)]
    pub choices: Vec<RuntimeRequestChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub questions: Option<Vec<RuntimeQuestion>>,
    #[serde(default)]
    pub supported: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRequestChoice {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Readable extent of an approval that outlives this request, such as a saved rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// One question. A provider offers any subset of the features: options, several selections, a
/// typed answer alongside or instead of the options, and secret input.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeQuestion {
    pub id: String,
    /// Short label shown above the question.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<RuntimeQuestionOption>>,
    /// Several options may be selected; each selection is one answer value.
    #[serde(default)]
    pub multi_select: bool,
    /// A typed answer is accepted, alongside the options when there are any.
    #[serde(default)]
    pub is_other: bool,
    #[serde(default)]
    pub is_secret: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeQuestionOption {
    pub label: String,
    #[serde(default)]
    pub description: String,
}

impl RuntimeQuestion {
    /// Checks one question's answer values against what the question offers.
    pub fn validate_answer(&self, values: &[String]) -> Result<(), &'static str> {
        if values.is_empty() || values.iter().any(|value| value.is_empty()) {
            return Err("Question answers must contain text");
        }
        if values.len() > 1 && !self.multi_select {
            return Err("Choose one answer for this question");
        }
        let offered = self.options.as_deref().unwrap_or_default();
        if !self.is_other
            && !offered.is_empty()
            && values
                .iter()
                .any(|value| !offered.iter().any(|option| &option.label == value))
        {
            return Err("Answer is outside the offered choices");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_requests_already_stored_in_history() {
        let stored = json!({"id":"approval","kind":"approval","title":"Approval requested","command":null,"cwd":null,"url":null,"permissions":null,"grantRoot":null,"choices":[{"id":"choice-1","label":"Allow once","description":"Run this command once.","scope":null}],"questions":null,"supported":true});
        let request: RuntimeRequest = serde_json::from_value(stored).unwrap();
        assert_eq!(request.kind, RuntimeRequestKind::Approval);
        assert_eq!(request.choices[0].label, "Allow once");
        let minimal: RuntimeRequest =
            serde_json::from_value(json!({"id":"question","kind":"questions"})).unwrap();
        assert_eq!(minimal.kind, RuntimeRequestKind::Questions);
    }

    #[test]
    fn answers_respect_selection_count_options_and_typed_answers() {
        let question = RuntimeQuestion {
            id: "q".into(),
            question: "Pick".into(),
            options: Some(vec![
                RuntimeQuestionOption { label: "One".into(), description: String::new() },
                RuntimeQuestionOption { label: "Two".into(), description: String::new() },
            ]),
            ..Default::default()
        };
        assert!(question.validate_answer(&["One".into()]).is_ok());
        assert!(question.validate_answer(&["One".into(), "Two".into()]).is_err());
        assert!(question.validate_answer(&["Three".into()]).is_err());
        let broad = RuntimeQuestion { multi_select: true, is_other: true, ..question };
        assert!(broad.validate_answer(&["One".into(), "Two".into()]).is_ok());
        assert!(broad.validate_answer(&["Typed".into()]).is_ok());
        assert!(broad.validate_answer(&[]).is_err());
    }
}
