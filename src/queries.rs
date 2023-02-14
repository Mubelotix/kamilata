
#[derive(Debug, Clone)]
pub struct SearchQueries {
    pub(crate) inner: Vec<(Vec<String>, usize)>,
}

impl From<Vec<String>> for SearchQueries {
    fn from(words: Vec<String>) -> Self {
        let words_len = words.len();
        SearchQueries { inner: vec![(words, words_len)] }
    }
}

impl From<Vec<&str>> for SearchQueries {
    fn from(words: Vec<&str>) -> Self {
        let words: Vec<String> = words.iter().map(|s| s.to_string()).collect();
        let words_len = words.len();
        SearchQueries { inner: vec![(words, words_len)] }
    }
}

impl From<&str> for SearchQueries {
    fn from(query: &str) -> Self {
        let words: Vec<String> = query.split_whitespace().filter(|s| s.len() >= 3).map(|s| s.to_string()).collect();
        let words_len = words.len();
        SearchQueries {
            inner: vec![(words, words_len)]
        }
    }
}

impl From<String> for SearchQueries {
    fn from(query: String) -> Self {
        Self::from(query.as_str())
    }
}

impl From<Vec<(Vec<String>, usize)>> for SearchQueries {
    fn from(queries: Vec<(Vec<String>, usize)>) -> Self {
        SearchQueries { inner: queries }
    }
}
