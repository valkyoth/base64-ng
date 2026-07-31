use std::process;

pub(crate) fn env_id(name: &str, default: &str) -> String {
    let value = std::env::var(name).unwrap_or_else(|_| default.to_owned());
    if is_evidence_id(&value) {
        value
    } else {
        eprintln!("{name} must match [A-Za-z0-9][A-Za-z0-9._-]{{0,63}}");
        process::exit(2);
    }
}

fn is_evidence_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::is_evidence_id;

    #[test]
    fn accepts_stable_machine_labels() {
        for value in ["run-1", "commit.5_host", "A", "z9"] {
            assert!(is_evidence_id(value), "{value}");
        }
    }

    #[test]
    fn rejects_structural_and_formula_labels() {
        for value in [
            "",
            "=formula",
            "+formula",
            "-formula",
            "@formula",
            "comma,value",
            "line\nvalue",
            " space",
        ] {
            assert!(!is_evidence_id(value), "{value:?}");
        }
        assert!(!is_evidence_id(&"a".repeat(65)));
    }
}
