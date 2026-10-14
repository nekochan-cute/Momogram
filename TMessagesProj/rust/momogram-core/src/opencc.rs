use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};

type Dict = BTreeMap<String, String>;

struct Stage {
    automaton: AhoCorasick,
    replacements: Vec<String>,
}

struct Converter {
    stages: Vec<Stage>,
}

macro_rules! embedded_dicts {
    ($( $static_name:ident => ($name:literal, $path:literal) ),+ $(,)?) => {
        $(
            static $static_name: Lazy<Dict> =
                Lazy::new(|| parse_dictionary(include_str!($path)));
        )+

        #[cfg(test)]
        const DICT_SOURCES: &[(&str, &str)] = &[
            $(
                ($name, include_str!($path)),
            )+
        ];

        fn dictionary(name: &str) -> Option<&'static Dict> {
            match name {
                $(
                    $name => Some(&*$static_name),
                )+
                _ => None,
            }
        }
    };
}

embedded_dicts! {
    HK_VARIANTS_REV_PHRASES => ("HKVariantsRevPhrases", "../../../src/main/assets/dictionary/HKVariantsRevPhrases.txt"),
    HK_VARIANTS_REV => ("HKVariantsRev", "../../../src/main/assets/dictionary/HKVariantsRev.txt"),
    HK_VARIANTS => ("HKVariants", "../../../src/main/assets/dictionary/HKVariants.txt"),
    JP_SHINJITAI_CHARACTERS => ("JPShinjitaiCharacters", "../../../src/main/assets/dictionary/JPShinjitaiCharacters.txt"),
    JP_SHINJITAI_PHRASES => ("JPShinjitaiPhrases", "../../../src/main/assets/dictionary/JPShinjitaiPhrases.txt"),
    JP_VARIANTS_REV => ("JPVariantsRev", "../../../src/main/assets/dictionary/JPVariantsRev.txt"),
    JP_VARIANTS => ("JPVariants", "../../../src/main/assets/dictionary/JPVariants.txt"),
    ST_CHARACTERS => ("STCharacters", "../../../src/main/assets/dictionary/STCharacters.txt"),
    ST_PHRASES => ("STPhrases", "../../../src/main/assets/dictionary/STPhrases.txt"),
    TS_CHARACTERS => ("TSCharacters", "../../../src/main/assets/dictionary/TSCharacters.txt"),
    TS_PHRASES => ("TSPhrases", "../../../src/main/assets/dictionary/TSPhrases.txt"),
    TW_PHRASES_IT => ("TWPhrasesIT", "../../../src/main/assets/dictionary/TWPhrasesIT.txt"),
    TW_PHRASES_NAME => ("TWPhrasesName", "../../../src/main/assets/dictionary/TWPhrasesName.txt"),
    TW_PHRASES_OTHER => ("TWPhrasesOther", "../../../src/main/assets/dictionary/TWPhrasesOther.txt"),
    TW_PHRASES_REV => ("TWPhrasesRev", "../../../src/main/assets/dictionary/TWPhrasesRev.txt"),
    TW_VARIANTS_REV_PHRASES => ("TWVariantsRevPhrases", "../../../src/main/assets/dictionary/TWVariantsRevPhrases.txt"),
    TW_VARIANTS_REV => ("TWVariantsRev", "../../../src/main/assets/dictionary/TWVariantsRev.txt"),
    TW_VARIANTS => ("TWVariants", "../../../src/main/assets/dictionary/TWVariants.txt"),
}
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
        stages.push(build_stage(names)?);
    }
    Ok(Converter { stages })
}

fn build_stage(names: &[&str]) -> Result<Stage, String> {
    let mut dicts = Vec::with_capacity(names.len());
    for name in names {
        dicts.push(
            dictionary(name)
                .ok_or_else(|| format!("OpenCC dictionary {name} is missing"))?,
        );
    }
    build_stage_from_dicts(&dicts)
}

fn build_stage_from_dicts(dicts: &[&Dict]) -> Result<Stage, String> {
    let mut merged = Dict::new();
    for dict in dicts {
        for (key, value) in *dict {
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
    fn embedded_dictionary_source_format_is_well_formed() {
        for (name, data) in DICT_SOURCES {
            for (index, line) in data.lines().enumerate() {
                if line.is_empty() {
                    continue;
                }
                let (key, _) = line
                    .split_once('\t')
                    .unwrap_or_else(|| panic!("{name}:{} is missing a tab", index + 1));
                assert!(!key.is_empty(), "{name}:{} has an empty key", index + 1);
            }
        }
    }

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
        let first = parse_dictionary("A\tfirst\n");
        let second = parse_dictionary("A\tsecond\n");
        let longest = parse_dictionary("Q\tshort\nQR\tlong\n");

        let converter = Converter {
            stages: vec![
                build_stage_from_dicts(&[&first, &second]).unwrap(),
                build_stage_from_dicts(&[&longest]).unwrap(),
            ],
        };
        assert_eq!(apply_converter(&converter, "A QR"), "second long");
    }
}
