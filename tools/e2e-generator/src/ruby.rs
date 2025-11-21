use crate::fixtures::{Assertions, Fixture};
use anyhow::{Context, Result, bail};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs;

const RUBY_HELPERS_TEMPLATE: &str = r#"# frozen_string_literal: true

# rubocop:disable Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/MethodLength, Metrics/PerceivedComplexity, Metrics/ParameterLists, Style/Documentation, Style/IfUnlessModifier, Layout/LineLength, Layout/EmptyLineAfterGuardClause

require 'json'
require 'pathname'
require 'rspec/expectations'
require 'kreuzberg'
require 'rspec/core'

module E2ERuby
  module_function

  WORKSPACE_ROOT = Pathname.new(__dir__).join('..', '..', '..').expand_path
  TEST_DOCUMENTS = WORKSPACE_ROOT.join('test_documents')

  def resolve_document(relative)
    TEST_DOCUMENTS.join(relative)
  end

  def build_config(raw)
    return nil unless raw.is_a?(Hash) && !raw.empty?

    symbolize_keys(raw)
  end

  def symbolize_keys(value)
    case value
    when Hash
      value.each_with_object({}) do |(key, val), acc|
        symbol_key = key.respond_to?(:to_sym) ? key.to_sym : key
        acc[symbol_key] = symbolize_keys(val)
      end
    when Array
      value.map { |item| symbolize_keys(item) }
    else
      value
    end
  end

  def skip_reason_for(error, fixture_id, requirements, notes = nil)
    message = error.message.to_s
    downcased = message.downcase
    requirement_hit = requirements.any? { |req| downcased.include?(req.downcase) }
    missing_dependency = error.is_a?(Kreuzberg::Errors::MissingDependencyError) || downcased.include?('missing dependency')
    unsupported_format = downcased.include?('unsupported format')

    return nil unless missing_dependency || unsupported_format || requirement_hit

    reason =
      if missing_dependency
        dependency = error.respond_to?(:dependency) ? error.dependency : nil
        if dependency && !dependency.to_s.empty?
          "missing dependency #{dependency}"
        else
          'missing dependency'
        end
      elsif unsupported_format
        'unsupported format'
      elsif requirements.any?
        "requires #{requirements.join(', ')}"
      else
        'environmental requirement'
      end

    details = "Skipping #{fixture_id}: #{reason}. #{error.class}: #{message}"
    details += " Notes: #{notes}" if notes
    warn(details)
    details
  end

  def run_fixture(fixture_id, relative_path, config_hash, requirements:, notes:, skip_if_missing: true)
    document_path = resolve_document(relative_path)

    if skip_if_missing && !document_path.exist?
      warn "Skipping #{fixture_id}: missing document at #{document_path}"
      raise RSpec::Core::Pending::SkipDeclaredInExample, 'missing document'
    end

    config = build_config(config_hash)
    result = nil
    begin
      result = Kreuzberg.extract_file_sync(document_path.to_s, config: config)
    rescue StandardError => e
      if (reason = skip_reason_for(e, fixture_id, requirements, notes))
        raise RSpec::Core::Pending::SkipDeclaredInExample, reason
      end
      raise
    end

    yield result
  end

  module Assertions
    extend RSpec::Matchers

    def self.assert_expected_mime(result, expected)
      return if expected.empty?

      expect(expected.any? { |token| result.mime_type.include?(token) }).to be(true)
    end

    def self.assert_min_content_length(result, minimum)
      expect(result.content.length).to be >= minimum
    end

    def self.assert_max_content_length(result, maximum)
      expect(result.content.length).to be <= maximum
    end

    def self.assert_content_contains_any(result, snippets)
      return if snippets.empty?

      lowered = result.content.downcase
      expect(snippets.any? { |snippet| lowered.include?(snippet.downcase) }).to be(true)
    end

    def self.assert_content_contains_all(result, snippets)
      return if snippets.empty?

      lowered = result.content.downcase
      expect(snippets.all? { |snippet| lowered.include?(snippet.downcase) }).to be(true)
    end

    def self.assert_table_count(result, minimum, maximum)
      tables = Array(result.tables)
      expect(tables.length).to be >= minimum if minimum
      expect(tables.length).to be <= maximum if maximum
    end

    def self.assert_detected_languages(result, expected, min_confidence)
      return if expected.empty?

      languages = result.detected_languages
      expect(languages).not_to be_nil
      expect(expected.all? { |lang| languages.include?(lang) }).to be(true)

      return unless min_confidence

      metadata = result.metadata || {}
      confidence = metadata['confidence'] || metadata[:confidence]
      expect(confidence).to be >= min_confidence if confidence
    end

    def self.assert_metadata_expectation(result, path, expectation)
      metadata = result.metadata || {}
      value = fetch_metadata_value(metadata, path)
      raise "Metadata path '#{path}' missing in #{metadata.inspect}" if value.nil?

      if expectation.key?(:eq)
        expect(values_equal?(value, expectation[:eq])).to be(true)
      end

      if expectation.key?(:gte)
        expect(convert_numeric(value)).to be >= convert_numeric(expectation[:gte])
      end

      if expectation.key?(:lte)
        expect(convert_numeric(value)).to be <= convert_numeric(expectation[:lte])
      end

      return unless expectation.key?(:contains)

      contains = expectation[:contains]
      if value.is_a?(String) && contains.is_a?(String)
        expect(value.include?(contains)).to be(true)
      elsif value.is_a?(Array) && contains.is_a?(Array)
        expect(contains.all? { |item| value.include?(item) }).to be(true)
      else
        raise "Unsupported contains expectation for path '#{path}'"
      end
    end

    class << self
      private

      def fetch_metadata_value(metadata, path)
        current = metadata
        path.split('.').each do |segment|
          return nil unless current.is_a?(Hash)

          current = current[segment] || current[segment.to_sym]
        end
        current
      end

      def values_equal?(lhs, rhs)
        return lhs == rhs if lhs.is_a?(String) && rhs.is_a?(String)
        return convert_numeric(lhs) == convert_numeric(rhs) if numeric_like?(lhs) && numeric_like?(rhs)
        return lhs == rhs if lhs == rhs

        lhs == rhs
      end

      def numeric_like?(value)
        value.is_a?(Numeric) || value.respond_to?(:to_f)
      end

      def convert_numeric(value)
        return value if value.is_a?(Numeric)

        value.to_f
      end
    end
  end
end
# rubocop:enable Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/MethodLength, Metrics/PerceivedComplexity, Metrics/ParameterLists, Style/Documentation, Style/IfUnlessModifier, Layout/LineLength, Layout/EmptyLineAfterGuardClause
"#;

const RUBY_SPEC_HELPER_TEMPLATE: &str = r#"# frozen_string_literal: true

require 'bundler/setup'
require 'rspec'
require_relative 'helpers'

RSpec.configure do |config|
  config.order = :defined
end
"#;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let ruby_root = output_root.join("ruby");
    let spec_dir = ruby_root.join("spec");

    fs::create_dir_all(&spec_dir).context("Failed to create Ruby spec directory")?;

    write_helpers(&spec_dir)?;
    write_spec_helper(&spec_dir)?;
    clean_spec_files(&spec_dir)?;

    // Separate document extraction and plugin API fixtures
    let doc_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_document_extraction()).collect();

    let plugin_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    // Generate document extraction tests
    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let file_name = format!("{}_spec.rb", sanitize_identifier(&category));
        let content = render_category(&category, &fixtures)?;
        let path = spec_dir.join(file_name);
        fs::write(&path, content).with_context(|| format!("Writing {}", path))?;
    }

    // Generate plugin API tests
    if !plugin_fixtures.is_empty() {
        generate_plugin_api_tests(&plugin_fixtures, &spec_dir)?;
    }

    Ok(())
}

fn clean_spec_files(spec_dir: &Utf8Path) -> Result<()> {
    if !spec_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(spec_dir.as_std_path())? {
        let entry = entry?;
        if entry
            .path()
            .file_name()
            .is_some_and(|name| name == "helpers.rb" || name == "spec_helper.rb")
        {
            continue;
        }
        if entry.path().extension().is_some_and(|ext| ext == "rb") {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn write_helpers(spec_dir: &Utf8Path) -> Result<()> {
    let helpers_path = spec_dir.join("helpers.rb");
    fs::write(&helpers_path, RUBY_HELPERS_TEMPLATE).context("Failed to write Ruby helpers")
}

fn write_spec_helper(spec_dir: &Utf8Path) -> Result<()> {
    let spec_helper_path = spec_dir.join("spec_helper.rb");
    fs::write(&spec_helper_path, RUBY_SPEC_HELPER_TEMPLATE).context("Failed to write Ruby spec_helper")
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "# frozen_string_literal: true")?;
    writeln!(buffer)?;
    writeln!(buffer, "# Auto-generated tests for {category} fixtures.")?;
    writeln!(buffer)?;
    writeln!(
        buffer,
        "# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength"
    )?;
    writeln!(buffer, "require_relative 'spec_helper'\n")?;
    writeln!(
        buffer,
        "RSpec.describe {} do",
        render_ruby_string(&format!("{category} fixtures"))
    )?;

    for (index, fixture) in fixtures.iter().enumerate() {
        let is_last = index == fixtures.len() - 1;
        buffer.push_str(&render_example(fixture, is_last)?);
    }

    writeln!(buffer, "end")?;
    writeln!(
        buffer,
        "# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength"
    )?;
    Ok(buffer)
}

fn render_example(fixture: &Fixture, is_last: bool) -> Result<String> {
    let mut body = String::new();

    writeln!(body, "  it {} do", render_ruby_string(&fixture.id))?;
    writeln!(body, "    E2ERuby.run_fixture(")?;
    writeln!(body, "      {},", render_ruby_string(&fixture.id))?;
    writeln!(body, "      {},", render_ruby_string(&fixture.document().path))?;

    let config_expr = render_config_expression(&fixture.extraction().config)?;
    match config_expr {
        None => writeln!(body, "      nil,")?,
        Some(expr) => writeln!(body, "      {expr},")?,
    }

    let requirements = render_string_array(&collect_requirements(fixture));
    let notes_literal = render_optional_string(fixture.skip().notes.as_ref());
    writeln!(body, "      requirements: {},", requirements)?;
    writeln!(body, "      notes: {},", notes_literal)?;
    let skip_flag = if fixture.skip().if_document_missing {
        "true"
    } else {
        "false"
    };
    writeln!(body, "      skip_if_missing: {}", skip_flag)?;
    writeln!(body, "    ) do |result|")?;

    let assertions = render_assertions(&fixture.assertions());
    if !assertions.is_empty() {
        body.push_str(&assertions);
    }

    writeln!(body, "    end")?;
    writeln!(body, "  end")?;
    if !is_last {
        writeln!(body)?;
    }

    Ok(body)
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        buffer.push_str("      E2ERuby::Assertions.assert_expected_mime(\n");
        buffer.push_str("        result,\n");
        buffer.push_str(&format!("        {}\n", render_string_array(&assertions.expected_mime)));
        buffer.push_str("      )\n");
    }

    if let Some(min) = assertions.min_content_length {
        buffer.push_str(&format!(
            "      E2ERuby::Assertions.assert_min_content_length(result, {})\n",
            render_numeric_literal(min as u64)
        ));
    }

    if let Some(max) = assertions.max_content_length {
        buffer.push_str(&format!(
            "      E2ERuby::Assertions.assert_max_content_length(result, {})\n",
            render_numeric_literal(max as u64)
        ));
    }

    if !assertions.content_contains_any.is_empty() {
        buffer.push_str(&format!(
            "      E2ERuby::Assertions.assert_content_contains_any(result, {})\n",
            render_string_array(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buffer.push_str(&format!(
            "      E2ERuby::Assertions.assert_content_contains_all(result, {})\n",
            render_string_array(&assertions.content_contains_all)
        ));
    }

    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables
            .min
            .map(|value| render_numeric_literal(value as u64))
            .unwrap_or_else(|| "nil".into());
        let max_literal = tables
            .max
            .map(|value| render_numeric_literal(value as u64))
            .unwrap_or_else(|| "nil".into());
        buffer.push_str(&format!(
            "      E2ERuby::Assertions.assert_table_count(result, {min}, {max})\n",
            min = min_literal,
            max = max_literal
        ));
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_array(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|value| value.to_string())
            .unwrap_or_else(|| "nil".into());
        buffer.push_str(&format!(
            "      E2ERuby::Assertions.assert_detected_languages(result, {expected}, {min_conf})\n"
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buffer.push_str(&format!(
                "      E2ERuby::Assertions.assert_metadata_expectation(result, {}, {})\n",
                render_ruby_string(path),
                render_ruby_value(expectation)
            ));
        }
    }

    buffer
}

fn render_config_expression(config: &Map<String, Value>) -> Result<Option<String>> {
    if config.is_empty() {
        Ok(None)
    } else {
        let value = Value::Object(config.clone());
        Ok(Some(render_ruby_value(&value)))
    }
}

fn render_ruby_value(value: &Value) -> String {
    match value {
        Value::Null => "nil".into(),
        Value::Bool(bool) => {
            if *bool {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(number) => render_number_value(number),
        Value::String(text) => render_ruby_string(text),
        Value::Array(items) => {
            if items.is_empty() {
                "[]".into()
            } else {
                let inner = items.iter().map(render_ruby_value).collect::<Vec<_>>().join(", ");
                format!("[{inner}]")
            }
        }
        Value::Object(map) => render_ruby_object(map),
    }
}

fn render_string_array(items: &[String]) -> String {
    if items.is_empty() {
        "[]".into()
    } else if items.iter().all(|item| is_simple_word(item)) {
        let joined = items.join(" ");
        format!("%w[{joined}]")
    } else {
        let content = items
            .iter()
            .map(|item| render_ruby_string(item))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{content}]")
    }
}

fn render_optional_string(value: Option<&String>) -> String {
    match value {
        Some(text) => render_ruby_string(text),
        None => "nil".into(),
    }
}

fn render_ruby_string(text: &str) -> String {
    if text.contains('\n') || text.contains('\r') {
        let escaped = text
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        format!("\"{escaped}\"")
    } else if text.contains('\'') {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        let escaped = text.replace('\\', "\\\\");
        format!("'{escaped}'")
    }
}

fn sanitize_identifier(input: &str) -> String {
    let mut output = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if idx == 0 && ch.is_ascii_digit() {
                output.push('_');
            }
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() { "fixture".into() } else { output }
}

fn render_ruby_object(map: &Map<String, Value>) -> String {
    if map.is_empty() {
        return "{}".into();
    }

    let pairs = map
        .iter()
        .map(|(key, value)| {
            if is_symbol_key(key) {
                format!("{}: {}", key, render_ruby_value(value))
            } else {
                format!("{} => {}", render_ruby_string(key), render_ruby_value(value))
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {pairs} }}")
}

fn render_numeric_literal(value: u64) -> String {
    let digits = value.to_string();
    if digits.len() <= 3 {
        return digits;
    }

    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx != 0 && idx % 3 == 0 {
            output.push('_');
        }
        output.push(ch);
    }
    output.chars().rev().collect()
}

fn render_number_value(number: &serde_json::Number) -> String {
    if let Some(value) = number.as_u64() {
        render_numeric_literal(value)
    } else if let Some(value) = number.as_i64() {
        if value >= 0 {
            render_numeric_literal(value as u64)
        } else {
            let positive = render_numeric_literal(value.unsigned_abs());
            format!("-{positive}")
        }
    } else if let Some(value) = number.as_f64() {
        value.to_string()
    } else {
        number.to_string()
    }
}

fn is_simple_word(text: &str) -> bool {
    !text.is_empty()
        && !text.chars().any(char::is_whitespace)
        && text
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn is_symbol_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    let mut chars = key.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && first != '_' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch == '_' || ch.is_ascii_digit())
}

fn collect_requirements(fixture: &Fixture) -> Vec<String> {
    fixture
        .skip()
        .requires_feature
        .iter()
        .chain(fixture.document().requires_external_tool.iter())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

// Plugin API test generation

fn generate_plugin_api_tests(fixtures: &[&Fixture], spec_dir: &Utf8Path) -> Result<()> {
    let mut buffer = String::new();

    // File header
    writeln!(buffer, "# frozen_string_literal: true")?;
    writeln!(buffer)?;
    writeln!(buffer, "# Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(buffer)?;
    writeln!(buffer, "# E2E tests for plugin/config/utility APIs.")?;
    writeln!(buffer, "#")?;
    writeln!(buffer, "# Generated from plugin API fixtures.")?;
    writeln!(
        buffer,
        "# To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang ruby"
    )?;
    writeln!(buffer)?;
    writeln!(
        buffer,
        "# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "require 'spec_helper'")?;
    writeln!(buffer, "require 'tmpdir'")?;
    writeln!(buffer, "require 'fileutils'")?;
    writeln!(buffer)?;

    // Group fixtures by api_category
    let mut grouped = fixtures
        .iter()
        .into_group_map_by(|fixture| {
            fixture
                .api_category
                .as_ref()
                .expect("api_category required for plugin API fixtures")
                .clone()
        })
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate tests grouped by category
    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let category_title = to_title_case(&category);
        writeln!(buffer, "RSpec.describe '{}' do", category_title)?;

        for fixture in fixtures {
            buffer.push_str(&render_plugin_test(fixture)?);
        }

        writeln!(buffer, "end")?;
        writeln!(buffer)?;
    }

    writeln!(
        buffer,
        "# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength"
    )?;

    let path = spec_dir.join("plugin_apis_spec.rb");
    fs::write(&path, buffer).with_context(|| format!("Writing {}", path))?;

    Ok(())
}

fn render_plugin_test(fixture: &Fixture) -> Result<String> {
    let mut buffer = String::new();
    let test_spec = fixture
        .test_spec
        .as_ref()
        .expect("test_spec required for plugin API fixtures");

    // Generate test name from description
    let test_name = &fixture.description;
    writeln!(buffer, "  it '{}' do", escape_ruby_string_content(test_name))?;

    // Render based on pattern
    match test_spec.pattern.as_str() {
        "simple_list" => render_simple_list_test(&mut buffer, fixture)?,
        "clear_registry" => render_clear_registry_test(&mut buffer, fixture)?,
        "graceful_unregister" => render_graceful_unregister_test(&mut buffer, fixture)?,
        "config_from_file" => render_config_from_file_test(&mut buffer, fixture)?,
        "config_discover" => render_config_discover_test(&mut buffer, fixture)?,
        "mime_from_bytes" => render_mime_from_bytes_test(&mut buffer, fixture)?,
        "mime_from_path" => render_mime_from_path_test(&mut buffer, fixture)?,
        "mime_extension_lookup" => render_mime_extension_lookup_test(&mut buffer, fixture)?,
        _ => {
            bail!("Unknown plugin test pattern: {}", test_spec.pattern);
        }
    }

    writeln!(buffer, "  end")?;
    writeln!(buffer)?;

    Ok(buffer)
}

fn render_simple_list_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let function_name = &test_spec.function_call.name;

    writeln!(buffer, "    result = Kreuzberg.{}", function_name)?;
    writeln!(buffer, "    expect(result).to be_an(Array)")?;

    if let Some(item_type) = &test_spec.assertions.list_item_type {
        let ruby_type = match item_type.as_str() {
            "string" => "String",
            "number" => "Numeric",
            "boolean" => "Object", // Ruby doesn't have a Boolean class
            _ => "Object",
        };
        writeln!(buffer, "    expect(result).to all(be_a({}))", ruby_type)?;
    }

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(
            buffer,
            "    expect(result).to include('{}')",
            escape_ruby_string_content(contains)
        )?;
    }

    Ok(())
}

fn render_clear_registry_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let clear_function = &test_spec.function_call.name;

    // Extract the list function name by replacing 'clear_' with 'list_'
    let list_function = clear_function.replace("clear_", "list_");

    writeln!(buffer, "    Kreuzberg.{}", clear_function)?;

    if test_spec.assertions.verify_cleanup {
        writeln!(buffer, "    result = Kreuzberg.{}", list_function)?;
        writeln!(buffer, "    expect(result).to be_empty")?;
    }

    Ok(())
}

fn render_graceful_unregister_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let function_name = &test_spec.function_call.name;

    // Get the argument (should be the name of a nonexistent item)
    let arg = test_spec
        .function_call
        .args
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("nonexistent-item-xyz");

    writeln!(
        buffer,
        "    expect {{ Kreuzberg.{}('{}') }}.not_to raise_error",
        function_name,
        escape_ruby_string_content(arg)
    )?;

    Ok(())
}

fn render_config_from_file_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let setup = test_spec.setup.as_ref().expect("setup required for config_from_file");

    // Create temp file
    let temp_file_name = setup.temp_file_name.as_ref().expect("temp_file_name required");
    let temp_file_content = setup.temp_file_content.as_ref().expect("temp_file_content required");

    writeln!(buffer, "    Dir.mktmpdir do |tmpdir|")?;
    writeln!(
        buffer,
        "      config_path = File.join(tmpdir, '{}')",
        escape_ruby_string_content(temp_file_name)
    )?;
    writeln!(buffer, "      File.write(config_path, <<~TOML)")?;
    writeln!(buffer, "{}", temp_file_content)?;
    writeln!(buffer, "      TOML")?;
    writeln!(buffer)?;

    // Call ExtractionConfig.from_file (note: Ruby uses snake_case class method names)
    let class_name = test_spec
        .function_call
        .class_name
        .as_ref()
        .expect("class_name required");
    let ruby_class = map_ruby_class_name(class_name);
    let method_name = &test_spec.function_call.name;

    writeln!(
        buffer,
        "      config = Kreuzberg::Config::{}.{}(config_path)",
        ruby_class, method_name
    )?;
    writeln!(buffer)?;

    // Assertions
    for prop in &test_spec.assertions.object_properties {
        render_object_property_assertion(buffer, "config", prop, "      ")?;
    }

    writeln!(buffer, "    end")?;

    Ok(())
}

fn render_config_discover_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let setup = test_spec.setup.as_ref().expect("setup required for config_discover");

    let temp_file_name = setup.temp_file_name.as_ref().expect("temp_file_name required");
    let temp_file_content = setup.temp_file_content.as_ref().expect("temp_file_content required");
    let subdirectory_name = setup.subdirectory_name.as_ref().expect("subdirectory_name required");

    writeln!(buffer, "    Dir.mktmpdir do |tmpdir|")?;
    writeln!(
        buffer,
        "      config_path = File.join(tmpdir, '{}')",
        escape_ruby_string_content(temp_file_name)
    )?;
    writeln!(buffer, "      File.write(config_path, <<~TOML)")?;
    writeln!(buffer, "{}", temp_file_content)?;
    writeln!(buffer, "      TOML")?;
    writeln!(buffer)?;
    writeln!(
        buffer,
        "      subdir = File.join(tmpdir, '{}')",
        escape_ruby_string_content(subdirectory_name)
    )?;
    writeln!(buffer, "      FileUtils.mkdir_p(subdir)")?;
    writeln!(buffer)?;

    // Change directory and discover
    let class_name = test_spec
        .function_call
        .class_name
        .as_ref()
        .expect("class_name required");
    let ruby_class = map_ruby_class_name(class_name);
    let method_name = &test_spec.function_call.name;

    writeln!(buffer, "      FileUtils.cd(subdir) do")?;
    writeln!(
        buffer,
        "        config = Kreuzberg::Config::{}.{}",
        ruby_class, method_name
    )?;
    writeln!(buffer)?;

    // Assertions
    for prop in &test_spec.assertions.object_properties {
        render_object_property_assertion(buffer, "config", prop, "        ")?;
    }

    writeln!(buffer, "      end")?;
    writeln!(buffer, "    end")?;

    Ok(())
}

fn render_mime_from_bytes_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let setup = test_spec.setup.as_ref().expect("setup required for mime_from_bytes");

    let test_data = setup.test_data.as_ref().expect("test_data required");

    let function_name = &test_spec.function_call.name;

    // Convert test data to Ruby string with proper encoding
    writeln!(
        buffer,
        "    test_bytes = '{}'.dup.force_encoding('ASCII-8BIT')",
        escape_ruby_string_content(test_data)
    )?;
    writeln!(buffer, "    result = Kreuzberg.{}(test_bytes)", function_name)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buffer,
            "    expect(result.downcase).to include('{}')",
            escape_ruby_string_content(&contains.to_lowercase())
        )?;
    }

    Ok(())
}

fn render_mime_from_path_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let function_name = &test_spec.function_call.name;

    writeln!(buffer, "    Dir.mktmpdir do |tmpdir|")?;
    writeln!(buffer, "      test_file = File.join(tmpdir, 'test.txt')")?;
    writeln!(buffer, "      File.write(test_file, 'Hello, world!')")?;
    writeln!(buffer)?;
    writeln!(buffer, "      result = Kreuzberg.{}(test_file)", function_name)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buffer,
            "      expect(result.downcase).to include('{}')",
            escape_ruby_string_content(&contains.to_lowercase())
        )?;
    }

    writeln!(buffer, "    end")?;

    Ok(())
}

fn render_mime_extension_lookup_test(buffer: &mut String, fixture: &Fixture) -> Result<()> {
    let test_spec = fixture.test_spec.as_ref().expect("test_spec required");
    let function_name = &test_spec.function_call.name;

    // Get the MIME type argument
    let mime_type = test_spec
        .function_call
        .args
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("application/pdf");

    writeln!(
        buffer,
        "    result = Kreuzberg.{}('{}')",
        function_name,
        escape_ruby_string_content(mime_type)
    )?;
    writeln!(buffer, "    expect(result).to be_an(Array)")?;

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(
            buffer,
            "    expect(result).to include('{}')",
            escape_ruby_string_content(contains)
        )?;
    }

    Ok(())
}

fn render_object_property_assertion(
    buffer: &mut String,
    var_name: &str,
    prop: &crate::fixtures::ObjectPropertyAssertion,
    indent: &str,
) -> Result<()> {
    let path_parts: Vec<&str> = prop.path.split('.').collect();
    let ruby_path = path_parts.join(".");

    // Check existence if specified
    if let Some(exists) = prop.exists {
        if exists {
            writeln!(buffer, "{}expect({}.{}).not_to be_nil", indent, var_name, ruby_path)?;
        } else {
            writeln!(buffer, "{}expect({}.{}).to be_nil", indent, var_name, ruby_path)?;
        }
    }

    // Check value if specified
    if let Some(value) = &prop.value {
        match value {
            Value::Number(n) => {
                writeln!(buffer, "{}expect({}.{}).to eq({})", indent, var_name, ruby_path, n)?;
            }
            Value::Bool(b) => {
                writeln!(buffer, "{}expect({}.{}).to eq({})", indent, var_name, ruby_path, b)?;
            }
            Value::String(s) => {
                writeln!(
                    buffer,
                    "{}expect({}.{}).to eq('{}')",
                    indent,
                    var_name,
                    ruby_path,
                    escape_ruby_string_content(s)
                )?;
            }
            _ => {
                writeln!(
                    buffer,
                    "{}expect({}.{}).to eq({})",
                    indent,
                    var_name,
                    ruby_path,
                    render_ruby_value(value)
                )?;
            }
        }
    }

    Ok(())
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_ruby_string_content(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// Map fixture class names to Ruby-specific class names
fn map_ruby_class_name(name: &str) -> &str {
    match name {
        "ExtractionConfig" => "Extraction",
        _ => name,
    }
}
