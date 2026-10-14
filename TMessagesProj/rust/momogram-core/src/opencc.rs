use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::OnceCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

type Dict = BTreeMap<String, String>;

struct Stage {
    automaton: AhoCorasick,
    replacements: Vec<String>,
}

struct Converter {
    stages: Vec<Stage>,
}

static DICTS: OnceCell<HashMap<String, Dict>> = OnceCell::new();
static CONVERTERS: OnceCell<Mutex<HashMap<i32, Arc<Converter>>>> = OnceCell::new();

pub fn init(bundle: &[u8]) -> Result<(), String> {
    if DICTS.get().is_some() {
        return Ok(());
    }

    let mut cursor = Cursor::new(bundle);
    let count = cursor.read_u32_be()? as usize;
    let mut dicts = HashMap::with_capacity(count);

    for _ in 0..count {
        let name_len = cursor.read_u32_be()? as usize;
        let name = cursor.read_utf8(name_len)?;
        let data_len = cursor.read_u32_be()? as usize;
        let data = cursor.read_utf8(data_len)?;
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
        dicts.insert(name, map);
    }

    if cursor.remaining() != 0 {
        return Err("OpenCC dictionary bundle has trailing bytes".to_owned());
    }

    let _ = DICTS.set(dicts);
    let _ = CONVERTERS.set(Mutex::new(HashMap::new()));
    Ok(())
}

pub fn convert(target: i32, input: &str) -> Result<String, String> {
    let converter = get_converter(target)?;
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
    Ok(result)
}

fn get_converter(target: i32) -> Result<Arc<Converter>, String> {
    let converters = CONVERTERS
        .get()
        .ok_or_else(|| "OpenCC dictionaries are not initialized".to_owned())?;
    if let Some(existing) = converters
        .lock()
        .map_err(|_| "OpenCC converter lock poisoned")?
        .get(&target)
        .cloned()
    {
        return Ok(existing);
    }

    let converter = Arc::new(build_converter(target)?);
    converters
        .lock()
        .map_err(|_| "OpenCC converter lock poisoned")?
        .insert(target, converter.clone());
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

    let dicts = DICTS
        .get()
        .ok_or_else(|| "OpenCC dictionaries are not initialized".to_owned())?;
    let mut stages = Vec::with_capacity(stage_names.len());

    for names in stage_names {
        let mut merged = Dict::new();
        for name in *names {
            let dict = dicts
                .get(*name)
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
        stages.push(Stage {
            automaton,
            replacements,
        });
    }

    Ok(Converter { stages })
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn read_u32_be(&mut self) -> Result<u32, String> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
    }

    fn read_utf8(&mut self, len: usize) -> Result<String, String> {
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(|s| s.to_owned())
            .map_err(|e| format!("Invalid UTF-8 in OpenCC dictionary bundle: {e}"))
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| "OpenCC dictionary bundle length overflow".to_owned())?;
        if end > self.data.len() {
            return Err("Truncated OpenCC dictionary bundle".to_owned());
        }
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_and_longest_match_work() {
        let dicts = [
            ("JPShinjitaiCharacters", "台\t臺\n"),
            ("JPVariantsRev", ""),
            ("TSCharacters", "臺\t台\n灣\t湾\n"),
        ];
        let mut bundle = Vec::new();
        bundle.extend_from_slice(&(dicts.len() as u32).to_be_bytes());
        for (name, data) in dicts {
            bundle.extend_from_slice(&(name.len() as u32).to_be_bytes());
            bundle.extend_from_slice(name.as_bytes());
            bundle.extend_from_slice(&(data.len() as u32).to_be_bytes());
            bundle.extend_from_slice(data.as_bytes());
        }
        init(&bundle).unwrap();
        assert_eq!(convert(0, "臺灣").unwrap(), "台湾");
    }
}
