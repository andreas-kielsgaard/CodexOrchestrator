use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn validate_model(model: &str) -> Result<(), String> {
    if !model.starts_with("jev-")
        || model.len() <= 4
        || model.len() > 128
        || !model
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-._".contains(&c))
    {
        return Err("Jev model must be a Jev model ID or alias".into());
    }
    Ok(())
}

/// One independent evaluation. The session supplies the model; callers supply all state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JevEvaluation {
    pub state: Value,
    pub questions: BTreeMap<String, JevQuestion>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum JevQuestion {
    Noul {
        instructions: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        criteria: Option<JevNoulCriteria>,
    },
    Choice {
        instructions: Value,
        criteria: BTreeMap<String, Value>,
    },
    Score {
        instructions: Value,
        criteria: Vec<Value>,
    },
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JevNoulCriteria {
    #[serde(rename = "true", default, skip_serializing_if = "Option::is_none")]
    pub yes: Option<Value>,
    #[serde(rename = "false", default, skip_serializing_if = "Option::is_none")]
    pub no: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JevResult {
    pub model: String,
    pub answers: BTreeMap<String, JevAnswer>,
    pub usage: JevUsage,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JevAnswer {
    Noul {
        noul: f64,
    },
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
    Score {
        score: f64,
        legend: BTreeMap<String, Value>,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JevUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

fn text_value(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Object(_) | Value::Array(_))
}

impl JevEvaluation {
    pub fn validate(&self) -> Result<(), String> {
        if !text_value(&self.state) || self.questions.is_empty() {
            return Err(
                "Jev requires string, object, or array state and at least one question".into(),
            );
        }
        for (id, question) in &self.questions {
            if id.trim().is_empty() {
                return Err("Jev question IDs cannot be empty".into());
            }
            let (instructions, valid_criteria) = match question {
                JevQuestion::Noul {
                    instructions,
                    criteria,
                } => (
                    instructions,
                    criteria
                        .as_ref()
                        .is_none_or(|c| c.yes.iter().chain(c.no.iter()).all(text_value)),
                ),
                JevQuestion::Choice {
                    instructions,
                    criteria,
                } => (
                    instructions,
                    !criteria.is_empty()
                        && criteria.len() <= 255
                        && criteria.iter().all(|(key, value)| {
                            !key.trim().is_empty() && (value.is_null() || text_value(value))
                        }),
                ),
                JevQuestion::Score {
                    instructions,
                    criteria,
                } => (
                    instructions,
                    (2..=10).contains(&criteria.len()) && criteria.iter().all(text_value),
                ),
            };
            if !text_value(instructions) || !valid_criteria {
                return Err(format!(
                    "Jev question {id} has invalid instructions or criteria"
                ));
            }
        }
        Ok(())
    }
}

fn probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn distribution(values: &BTreeMap<String, f64>) -> bool {
    values.values().copied().all(probability) && (values.values().sum::<f64>() - 1.0).abs() < 0.001
}

impl JevResult {
    pub fn validate_for(&self, evaluation: &JevEvaluation) -> Result<(), String> {
        if self.model.trim().is_empty() || self.answers.len() != evaluation.questions.len() {
            return Err("Jev returned an incomplete result".into());
        }
        for (id, question) in &evaluation.questions {
            let valid = match (question, self.answers.get(id)) {
                (JevQuestion::Noul { .. }, Some(JevAnswer::Noul { noul })) => probability(*noul),
                (
                    JevQuestion::Choice { criteria, .. },
                    Some(JevAnswer::Choice {
                        choice,
                        probabilities,
                        confidence,
                    }),
                ) => {
                    criteria.contains_key(choice)
                        && criteria.keys().eq(probabilities.keys())
                        && distribution(probabilities)
                        && probability(*confidence)
                }
                (
                    JevQuestion::Score { criteria, .. },
                    Some(JevAnswer::Score {
                        score,
                        legend,
                        probabilities,
                        confidence,
                    }),
                ) => {
                    let keys: BTreeMap<_, _> =
                        (0..criteria.len()).map(|i| (i.to_string(), ())).collect();
                    score.is_finite()
                        && (0.0..=(criteria.len() - 1) as f64).contains(score)
                        && keys.keys().eq(probabilities.keys())
                        && keys.keys().eq(legend.keys())
                        && distribution(probabilities)
                        && probability(*confidence)
                }
                _ => false,
            };
            if !valid {
                return Err(format!("Jev returned an invalid answer for question {id}"));
            }
        }
        Ok(())
    }
}
