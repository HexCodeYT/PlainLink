use plainlink::{RuleSet, clean_url};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct FixtureCase {
    name: String,
    input: String,
    expected: String,
    removed: Vec<String>,
}

#[test]
fn rule_fixtures_match_expected_outputs() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixture_paths = fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", fixture_dir.display()))
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .expect("could not read fixture entry");

    fixture_paths.sort();
    fixture_paths.retain(|path| {
        path.extension().and_then(|extension| extension.to_str()) == Some("plainlink-case")
    });

    assert!(
        !fixture_paths.is_empty(),
        "expected at least one PlainLink rule fixture"
    );

    let rules = RuleSet::default_rules();

    for path in fixture_paths {
        let fixture =
            parse_fixture(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let result = clean_url(&fixture.input, &rules);
        let removed = result
            .removed
            .iter()
            .map(|removed| removed.name.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            result.cleaned, fixture.expected,
            "{} cleaned URL",
            fixture.name
        );
        assert_eq!(removed, fixture.removed, "{} removed params", fixture.name);
    }
}

fn parse_fixture(path: &PathBuf) -> Result<FixtureCase, String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut name = None;
    let mut input = None;
    let mut expected = None;
    let mut removed = None;

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {line_number}: expected key = value"));
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "name" => name = Some(value.to_string()),
            "input" => input = Some(value.to_string()),
            "expected" => expected = Some(value.to_string()),
            "removed" => removed = Some(parse_removed_params(value)),
            other => return Err(format!("line {line_number}: unknown key `{other}`")),
        }
    }

    Ok(FixtureCase {
        name: name.ok_or("missing `name`")?,
        input: input.ok_or("missing `input`")?,
        expected: expected.ok_or("missing `expected`")?,
        removed: removed.unwrap_or_default(),
    })
}

fn parse_removed_params(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}
