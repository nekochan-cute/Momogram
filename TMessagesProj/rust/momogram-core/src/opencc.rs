use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};

type Dict = BTreeMap<String, String>;
type DictSet = HashMap<&'static str, Dict>;

struct Stage {
    automaton: AhoCorasick,
    replacements: Vec<String>,
}

struct Converter {
    stages: Vec<Stage>,
}

const DICT_SOURCES: &[(&str, &str)] = &[
    ("HKVariantsRevPhrases", include_str!("../../../src/main/assets/dictionary/HKVariantsRevPhrases.txt")),
    ("HKVariantsRev", include_str!("../../../src/main/assets/dictionary/HKVariantsRev.txt")),
    ("HKVariants", include_str!("../../../src/main/assets/dictionary/HKVariants.txt")),
    ("JPShinjitaiCharacters", include_str!("../../../src/main/assets/dictionary/JPShinjitaiCharacters.txt")),
    ("JPShinjitaiPhrases", include_str!("../../../src/main/assets/dictionary/JPShinjitaiPhrases.txt")),
    ("JPVariantsRev", include_str!("../../../src/main/assets/dictionary/JPVariantsRev.txt")),
    ("JPVariants", include_str!("../../../src/main/assets/dictionary/JPVariants.txt")),
    ("STCharacters", include_str!("../../../src/main/assets/dictionary/STCharacters.txt")),
    ("STPhrases", include_str!("../../../src/main/assets/dictionary/STPhrases.txt")),
    ("TSCharacters", include_str!("../../../src/main/assets/dictionary/TSCharacters.txt")),
    ("TSPhrases", include_str!("../../../src/main/assets/dictionary/TSPhrases.txt")),
    ("TWPhrasesIT", include_str!("../../../src/main/assets/dictionary/TWPhrasesIT.txt")),
    ("TWPhrasesName", include_str!("../../../src/main/assets/dictionary/TWPhrasesName.txt")),
    ("TWPhrasesOther", include_str!("../../../src/main/assets/dictionary/TWPhrasesOther.txt")),
    ("TWPhrasesRev", include_str!("../../../src/main/assets/dictionary/TWPhrasesRev.txt")),
    ("TWVariantsRevPhrases", include_str!("../../../src/main/assets/dictionary/TWVariantsRevPhrases.txt")),
    ("TWVariantsRev", include_str!("../../../src/main/assets/dictionary/TWVariantsRev.txt")),
    ("TWVariants", include_str!("../../../src/main/assets/dictionary/TWVariants.txt")),
];

static DICTS: Lazy<DictSet> = Lazy::new(|| {
    DICT_SOURCES
        .iter()
        .map(|(name, data)| (*name, parse_dictionary(data)))
        .collect()
});
static CONVERTERS: Lazy<Mutex<HashMap<i32, Arc<Converter>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn convert(target: i32, input: &str) -> Result<String, String> {
    let converter = get_converter(target)?;
    Ok(apply_converter(&converter, input))
}

fn apply_converter(converter: &Converter, input: &str) -> String {
    let mut result = input.to_owned();
    for stage in &converter.stages {
        let mut out = String::with_capacity(result.len());
        let mut last = 0;
        for m in stage.automaton.find_iter(result.as_bytes()) {
            out.push_str(&result[last..m.start()]);
            out.push_str(&stage.replacements[m.pattern().as_usize()]);
            last = m.end();
        }
        out.push_str(&result[last..]);
        result = out;
    }
    result
}

fn converter_lock() -> MutexGuard<'static, HashMap<i32, Arc<Converter>>> {
    CONVERTERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn get_converter(target: i32) -> Result<Arc<Converter>, String> {
    if let Some(existing) = converter_lock().get(&target).cloned() {
        return Ok(existing);
    }

    let converter = Arc::new(build_converter(target)?);
    converter_lock().insert(target, converter.clone());
    Ok(converter)
}

fn build_converter(target: i32) -> Result<Converter, String> {
    let stage_names: &[&[&str]] = match target {
        0 => &[
            &["JPShinjitaiCharacters", "JPVariantsRev"],
            &["TSCharacters"],
        ],
        1 => &[
            &[
                "JPShinjitaiCharacters",
                "JPShinjitaiPhrases",
                "JPVariantsRev",
                "HKVariantsRev",
                "HKVariantsRevPhrases",
                "TWVariantsRev",
                "TWPhrasesRev",
                "TWVariantsRevPhrases",
            ],
            &["TSCharacters", "TSPhrases"],
        ],
        2 => &[&[
            "JPShinjitaiCharacters",
            "JPVariantsRev",
            "STCharacters",
            "STPhrases",
        ]],
        3 => &[
            &[
                "JPShinjitaiCharacters",
                "JPShinjitaiPhrases",
                "JPVariantsRev",
                "STCharacters",
                "STPhrases",
            ],
            &["TWVariants", "TWPhrasesIT", "TWPhrasesName", "TWPhrasesOther"],
        ],
        4 => &[
            &[
                "JPShinjitaiCharacters",
                "JPShinjitaiPhrases",
                "JPVariantsRev",
                "STCharacters",
                "STPhrases",
            ],
            &["HKVariants"],
        ],
        5 => &[
            &["STCharacters", "STPhrases"],
            &["JPVariants", "JPShinjitaiCharacters", "JPShinjitaiPhrases"],
        ],
        _ => return Err(format!("Unknown OpenCC target {target}")),
    };

    let mut stages = Vec::with_capacity(stage_names.len());
    for names in stage_names {
        stages.push(build_stage(&DICTS, names)?);
    }
    Ok(Converter { stages })
}

fn build_stage(dicts: &DictSet, names: &[&str]) -> Result<Stage, String> {
    let mut merged = Dict::new();
    for name in names {
        let dict = dicts
            .get(name)
            .ok_or_else(|| format!("OpenCC dictionary {name} is missing"))?;
        for (key, value) in dict {
            merged.insert(key.clone(), value.clone());
        }
    }

    let patterns: Vec<String> = merged.keys().cloned().collect();
    let replacements: Vec<String> = merged.values().cloned().collect();
    let automaton = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&patterns)
        .map_err(|e| format!("OpenCC automaton build failed: {e}"))?;
    Ok(Stage {
        automaton,
        replacements,
    })
}

fn parse_dictionary(data: &str) -> Dict {
    let mut map = Dict::new();
    for line in data.lines() {
        let Some((key, values)) = line.split_once('\t') else {
            continue;
        };
        let Some(value) = values.split(' ').next() else {
            continue;
        };
        if !key.is_empty() {
            map.insert(key.to_owned(), value.to_owned());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_dictionaries_are_available_without_runtime_init() {
        assert_eq!(convert(0, "臺灣").unwrap(), "台湾");
    }

    #[test]
    fn all_embedded_targets_build() {
        for target in 0..=5 {
            convert(target, "測試").unwrap();
        }
        assert!(convert(6, "測試").is_err());
    }

    #[test]
    fn merge_order_and_longest_match_match_legacy_converter() {
        let mut dicts = DictSet::new();
        dicts.insert("first", parse_dictionary("A\tfirst\n"));
        dicts.insert("second", parse_dictionary("A\tsecond\n"));
        dicts.insert("longest", parse_dictionary("Q\tshort\nQR\tlong\n"));

        let converter = Converter {
            stages: vec![
                build_stage(&dicts, &["first", "second"]).unwrap(),
                build_stage(&dicts, &["longest"]).unwrap(),
            ],
        };
        assert_eq!(apply_converter(&converter, "A QR"), "second long");
    }
}
