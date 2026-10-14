use serde_json::Value;

const SCHEMAS: &str = include_str!("../schemas/commands.json");

pub fn command_ids() -> Vec<String> {
    let schemas: Value = serde_json::from_str(SCHEMAS).expect("commands.json");
    let mut ids: Vec<String> = schemas
        .as_object()
        .expect("schema map")
        .keys()
        .cloned()
        .collect();
    ids.sort();
    ids
}

pub fn validate(command: &str, data: &Value) -> Result<(), String> {
    let schemas: Value = serde_json::from_str(SCHEMAS).map_err(|err| err.to_string())?;
    let schema = schemas
        .get(command)
        .ok_or_else(|| format!("no schema for {command}"))?;
    check(schema, data).map_err(|err| format!("{command}: {err}"))
}

fn check(schema: &Value, data: &Value) -> Result<(), String> {
    let expected = schema
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("object");
    match expected {
        "object" => {
            let object = data.as_object().ok_or("expected object")?;
            if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
                for key in required {
                    let key = key.as_str().ok_or("required key must be a string")?;
                    if !object.contains_key(key) {
                        return Err(format!("missing {key}"));
                    }
                }
            }
            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, child) in properties {
                    if let Some(value) = object.get(key) {
                        check(child, value)?;
                    }
                }
            }
            Ok(())
        }
        "array" => {
            let items = data.as_array().ok_or("expected array")?;
            if let Some(item_schema) = schema.get("items") {
                for item in items {
                    check(item_schema, item)?;
                }
            }
            Ok(())
        }
        "string" => data.as_str().map(|_| ()).ok_or("expected string".into()),
        "boolean" => data.as_bool().map(|_| ()).ok_or("expected boolean".into()),
        "integer" => data.as_i64().map(|_| ()).ok_or("expected integer".into()),
        other => Err(format!("unsupported schema type {other}")),
    }
}
