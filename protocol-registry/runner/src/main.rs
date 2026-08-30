mod model;
mod production;
mod references;

const CORPUS: &str = include_str!("../../v1/cases.tsv");
const HEADER: &str = "id\tregistry_id\tdecision\tplain_hex\twire_hex\tsource_sha256\t\
errata_sha256\trequirement_ids\tprovenance_ids";

#[derive(Debug)]
pub(crate) struct Case {
    pub id: String,
    pub registry_id: String,
    pub accept: bool,
    pub plain: Vec<u8>,
    pub wire: Vec<u8>,
}

fn main() {
    let mut lines = CORPUS.lines();
    assert_eq!(lines.next(), Some(HEADER));
    let cases: Vec<_> = lines.map(parse_case).collect();
    assert!(!cases.is_empty());

    let mut protocol_cases = 0;
    for case in &cases {
        if case.registry_id == "core-config" {
            continue;
        }
        protocol_cases += 1;
        let modeled = model::evaluate(case);
        let produced = production::evaluate(case);
        assert_eq!(modeled.is_some(), case.accept, "{} model decision", case.id);
        assert_eq!(
            produced.is_some(),
            case.accept,
            "{} production decision",
            case.id
        );
        if case.accept && !case.plain.is_empty() {
            assert_eq!(
                modeled.as_deref(),
                Some(case.plain.as_slice()),
                "{} model",
                case.id
            );
            assert_eq!(
                produced.as_deref(),
                Some(case.plain.as_slice()),
                "{} production",
                case.id
            );
        }
    }

    references::run(&cases);
    println!(
        "protocol registry: {protocol_cases} protocol cases and pinned core references passed"
    );
}

fn parse_case(line: &str) -> Case {
    let columns: Vec<_> = line.split('\t').collect();
    assert_eq!(columns.len(), 9, "invalid corpus row: {line}");
    Case {
        id: columns[0].to_owned(),
        registry_id: columns[1].to_owned(),
        accept: match columns[2] {
            "accept" => true,
            "reject" => false,
            other => panic!("unknown decision {other}"),
        },
        plain: decode_hex(columns[3]),
        wire: decode_hex(columns[4]),
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
        .collect()
}

fn nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid corpus hex"),
    }
}
