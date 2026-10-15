use super::substitute_vars;
use serde_json::{Map, Value, json};

#[test]
fn should_substitute_every_json_value_kind_and_leave_null_literal() {
    let context = Map::from_iter([
        ("null".into(), Value::Null),
        ("true".into(), json!(true)),
        ("false".into(), json!(false)),
        ("number".into(), json!(1.5)),
        ("text".into(), json!("é")),
        ("array".into(), json!([1, "x"])),
        ("object".into(), json!({"nested":true})),
    ]);
    assert_eq!(context.len(), 7);
    assert_eq!(
        substitute_vars(
            "{{null}}|{{true}}|{{false}}|{{number}}|{{text}}|{{array}}|{{object}}",
            Some(&context)
        ),
        "{{null}}|true|false|1.5|é|[1,\"x\"]|{\"nested\":true}"
    );
}

#[test]
fn should_preserve_template_bytes_for_absent_empty_or_missing_context() {
    let template = "é {single} {{missing}} {{ unfinished} end";
    assert_eq!(substitute_vars(template, None), template);
    assert_eq!(substitute_vars(template, Some(&Map::new())), template);
    let context = Map::from_iter([("other".into(), json!("value"))]);
    assert_eq!(substitute_vars(template, Some(&context)), template);
}

#[test]
fn should_use_exact_whitespace_and_empty_keys_without_recursive_substitution() {
    let context = Map::from_iter([
        ("".into(), json!("empty")),
        ("key".into(), json!("trimmed")),
        (" key ".into(), json!("spaced")),
        ("line\nkey".into(), json!("雪\n{{key}}")),
        ("blank".into(), json!("")),
    ]);
    assert_eq!(
        substitute_vars("{{}}|{{ key }}|{{key}}|{{line\nkey}}|{{blank}}", Some(&context)),
        "empty|spaced|trimmed|雪\n{{key}}|"
    );
}

#[test]
fn should_preserve_nested_and_unclosed_brace_scanning_quirks() {
    let context = Map::from_iter([
        ("{x".into(), json!("inner")),
        ("a{{b".into(), json!("N")),
        ("x".into(), json!("X")),
        ("x}tail".into(), json!("unused")),
    ]);
    let cases = [
        ("{{{x}}}", "inner}"),
        ("{{a{{b}}c}}", "Nc}}"),
        ("before{{x}tail", "before{{x}tail"),
        ("}{{x}}{", "}X{"),
        ("{x}", "{x}"),
        ("{{", "{{"),
        ("{{x}", "{{x}"),
    ];
    assert_eq!(cases.len(), 7);
    for (template, expected) in cases {
        assert_eq!(substitute_vars(template, Some(&context)), expected, "{template}");
    }
}
