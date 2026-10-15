//! The extraction contract shared by every provider, the validator and the app.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::normalize::{de_confidence, de_money_opt, de_money_or_zero, de_qty, de_string_opt};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextField {
    #[serde(default, deserialize_with = "de_string_opt")]
    pub value: Option<String>,
    #[serde(default, deserialize_with = "de_confidence")]
    pub confidence: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MoneyField {
    #[serde(default, deserialize_with = "de_money_opt")]
    pub value: Option<i64>,
    #[serde(default, deserialize_with = "de_confidence")]
    pub confidence: f32,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    #[serde(default)]
    pub name: String,
    #[serde(default = "one", deserialize_with = "de_qty")]
    pub qty: f64,
    #[serde(default, deserialize_with = "de_money_opt")]
    pub unit_price: Option<i64>,
    #[serde(default, deserialize_with = "de_money_or_zero")]
    pub total: i64,
    #[serde(default, deserialize_with = "de_confidence")]
    pub confidence: f32,
    /// Set by the validator, never by the model. The app highlights flagged rows.
    #[serde(default)]
    pub flagged: bool,
}

fn default_true() -> bool {
    true
}

fn default_currency() -> String {
    "IDR".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Extraction {
    #[serde(default = "default_true")]
    pub is_receipt: bool,
    #[serde(default)]
    pub merchant: TextField,
    /// ISO 8601 as read from the receipt; parsed and checked by the validator.
    #[serde(default)]
    pub date: TextField,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub subtotal: MoneyField,
    #[serde(default)]
    pub tax: MoneyField,
    #[serde(default)]
    pub service: MoneyField,
    #[serde(default)]
    pub discount: MoneyField,
    #[serde(default)]
    pub total: MoneyField,
    #[serde(default)]
    pub category_suggestion: TextField,
    #[serde(default = "default_currency")]
    pub currency: String,
}

/// One problem found by code-side validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    /// "total", "date", "merchant", "items[2]", ...
    pub field: String,
    pub code: String,
    pub message: String,
}

impl Issue {
    pub fn new(field: impl Into<String>, code: &str, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Category slugs the model is allowed to suggest. Must match the seeded categories.
pub const CATEGORY_SLUGS: &[&str] = &[
    "food",
    "groceries",
    "shopping",
    "transport",
    "housing",
    "bills",
    "subscriptions",
    "health",
    "entertainment",
    "other",
];

fn text_field(desc: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "value": { "type": ["string", "null"], "description": desc },
            "confidence": { "type": "number" }
        },
        "required": ["value", "confidence"]
    })
}

fn money_field(desc: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "value": { "type": ["integer", "null"], "description": desc },
            "confidence": { "type": "number" }
        },
        "required": ["value", "confidence"]
    })
}

/// Standard JSON Schema, used for OpenAI-compatible `json_schema` mode and in the prompt.
pub fn json_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "is_receipt": { "type": "boolean" },
            "merchant": text_field("Store or restaurant name"),
            "date": text_field("ISO 8601 date-time, e.g. 2026-10-15T19:05:00+07:00"),
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "qty": { "type": "number" },
                        "unit_price": { "type": ["integer", "null"] },
                        "total": { "type": "integer" },
                        "confidence": { "type": "number" }
                    },
                    "required": ["name", "qty", "unit_price", "total", "confidence"]
                }
            },
            "subtotal": money_field("Sum before tax/discount, integer rupiah"),
            "tax": money_field("PPN / restaurant tax, integer rupiah"),
            "service": money_field("Service charge, integer rupiah"),
            "discount": money_field("Total discount as a positive integer rupiah"),
            "total": money_field("Amount actually owed (TOTAL), integer rupiah"),
            "category_suggestion": {
                "type": "object",
                "properties": {
                    "value": { "type": ["string", "null"], "enum": CATEGORY_SLUGS.iter().map(|s| Value::from(*s)).chain([Value::Null]).collect::<Vec<_>>() },
                    "confidence": { "type": "number" }
                },
                "required": ["value", "confidence"]
            },
            "currency": { "type": "string" }
        },
        "required": ["is_receipt", "merchant", "date", "items", "subtotal", "tax", "service", "discount", "total", "category_suggestion", "currency"]
    })
}

fn g_field(kind: &str) -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "value": { "type": kind, "nullable": true },
            "confidence": { "type": "NUMBER" }
        },
        "required": ["value", "confidence"]
    })
}

/// Gemini `responseSchema` (OpenAPI 3.0 subset: uppercase types, `nullable`).
pub fn gemini_schema() -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "is_receipt": { "type": "BOOLEAN" },
            "merchant": g_field("STRING"),
            "date": g_field("STRING"),
            "items": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "name": { "type": "STRING" },
                        "qty": { "type": "NUMBER" },
                        "unit_price": { "type": "INTEGER", "nullable": true },
                        "total": { "type": "INTEGER" },
                        "confidence": { "type": "NUMBER" }
                    },
                    "required": ["name", "qty", "unit_price", "total", "confidence"]
                }
            },
            "subtotal": g_field("INTEGER"),
            "tax": g_field("INTEGER"),
            "service": g_field("INTEGER"),
            "discount": g_field("INTEGER"),
            "total": g_field("INTEGER"),
            "category_suggestion": {
                "type": "OBJECT",
                "properties": {
                    "value": { "type": "STRING", "nullable": true, "enum": CATEGORY_SLUGS },
                    "confidence": { "type": "NUMBER" }
                },
                "required": ["value", "confidence"]
            },
            "currency": { "type": "STRING" }
        },
        "required": ["is_receipt", "merchant", "date", "items", "subtotal", "tax", "service", "discount", "total", "category_suggestion", "currency"],
        "propertyOrdering": ["is_receipt", "merchant", "date", "items", "subtotal", "discount", "tax", "service", "total", "category_suggestion", "currency"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lenient_parse_of_messy_model_output() {
        let raw = json!({
            "merchant": { "value": "  INDOMARET  ", "confidence": "0.9" },
            "items": [
                { "name": "INDOMIE GRG", "qty": "5", "unit_price": "Rp 3.500", "total": "17.500", "confidence": 1.4 }
            ],
            "total": { "value": "Rp 87.400", "confidence": 0.98 }
        });
        let e: Extraction = serde_json::from_value(raw).unwrap();
        assert_eq!(e.merchant.value.as_deref(), Some("INDOMARET"));
        assert_eq!(e.items[0].unit_price, Some(3_500));
        assert_eq!(e.items[0].total, 17_500);
        assert_eq!(e.items[0].confidence, 1.0);
        assert_eq!(e.total.value, Some(87_400));
        assert!(e.is_receipt);
        assert_eq!(e.currency, "IDR");
        assert_eq!(e.date.value, None);
    }
}
