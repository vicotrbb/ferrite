use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnsupportedFields {
    fields: Vec<String>,
}

impl UnsupportedFields {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn with_present(mut self, field: &'static str, present: bool) -> Self {
        if present {
            self.fields.push(field.to_owned());
        }
        self
    }

    pub fn with_extra_keys(mut self, extra_fields: &BTreeMap<String, Value>) -> Self {
        self.fields.extend(extra_fields.keys().cloned());
        self
    }

    pub fn with_extra_keys_with_prefix(
        mut self,
        prefix: &'static str,
        extra_fields: &BTreeMap<String, Value>,
    ) -> Self {
        self.fields
            .extend(extra_fields.keys().map(|field| format!("{prefix}{field}")));
        self
    }

    pub fn into_vec(self) -> Vec<String> {
        self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::UnsupportedFields;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn collects_named_and_extra_unsupported_fields() {
        let mut extra = BTreeMap::new();
        extra.insert("modalities".to_owned(), json!(["text"]));
        extra.insert("vendor_option".to_owned(), json!(true));

        let fields = UnsupportedFields::new()
            .with_present("temperature", true)
            .with_present("top_p", false)
            .with_extra_keys(&extra)
            .into_vec();

        assert_eq!(
            fields,
            vec![
                "temperature".to_owned(),
                "modalities".to_owned(),
                "vendor_option".to_owned()
            ]
        );
    }

    #[test]
    fn collects_prefixed_extra_unsupported_fields() {
        let mut extra = BTreeMap::new();
        extra.insert("vendor_context".to_owned(), json!({"trace": "local"}));

        let fields = UnsupportedFields::new()
            .with_extra_keys_with_prefix("messages.", &extra)
            .into_vec();

        assert_eq!(fields, vec!["messages.vendor_context".to_owned()]);
    }
}
