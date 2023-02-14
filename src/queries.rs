
#[derive(Debug, Clone)]
pub struct SearchQueries {
    pub(crate) inner: Vec<(Vec<String>, usize)>,
}

impl SearchQueries {
    pub(crate) fn from_inner(inner: Vec<(Vec<String>, usize)>) -> Self {
        Self {
            inner,
        }
    }
    
    pub fn from_raw_text(text: impl AsRef<str>) -> Self {
        let words: Vec<String> = text.as_ref().split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()).filter(|w| w.len() >= 3).map(|w| w.to_string()).collect();
        let words_len = words.len();
        Self {
            inner: vec![(words, words_len)],
        }
    }

    pub fn from_raw_text_iter(texts: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut inner = Vec::new();
        for text in texts {
            let words: Vec<String> = text.as_ref().split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()).filter(|w| w.len() >= 3).map(|w| w.to_string()).collect();
            let words_len = words.len();
            inner.push((words, words_len));
        }
        Self {
            inner,
        }
    }
}
