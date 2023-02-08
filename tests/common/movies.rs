use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Movie {
    id: usize,
    title: String,
    overview: String,
    genres: Vec<String>,
    poster: String,
    release_date: i64,
}

impl Movie {
    fn full_text(&self) -> String {
        let mut full_text = String::new();
        full_text.push_str(&self.title);
        full_text.push(' ');
        full_text.push_str(&self.overview);
        full_text.push(' ');
        for genre in &self.genres {
            full_text.push_str(genre);
            full_text.push(' ');
        }
        full_text = full_text.to_lowercase();
        full_text
    }

    pub fn words(&self) -> Vec<String> {
        self.full_text().split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()).filter(|w| w.len() >= 3).map(|w| w.to_string()).collect()
    }
}

impl SearchResult for Movie {
    type Cid = usize;

    fn cid(&self) -> &Self::Cid {
        &self.id
    }

    fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        serde_json::from_slice(bytes).unwrap()
    }
}

impl<const N: usize> Document<N> for Movie {
    type SearchResult = Movie;
    type WordHasher = WordHasherImpl<N>;

    fn cid(&self) -> &<Self::SearchResult as SearchResult>::Cid {
        &self.id
    }

    fn apply_to_filter(&self, filter: &mut Filter<N>) {
        self.words().iter().for_each(|word| {
            let idx = Self::WordHasher::hash_word(word);
            filter.set_bit(idx, true)
        });
    }

    fn search_result(&self, query_words: &[String], min_matching: usize) -> Option<Self::SearchResult> {
        let words = self.words();

        let mut matches = 0;
        for word in query_words {
            if words.contains(word) {
                matches += 1;
            }
        }

        if min_matching <= matches {
            Some(self.to_owned())
        } else {
            None
        }
    }
}

pub struct WordHasherImpl<const N: usize>;

impl<const N: usize> WordHasher<N> for WordHasherImpl<N> {
    fn hash_word(word: &str) -> usize {
        let mut result = 1usize;
        const RANDOM_SEED: [usize; 16] = [542587211452, 5242354514, 245421154, 4534542154, 542866467, 545245414, 7867569786914, 88797854597, 24542187316, 645785447, 434963879, 4234274, 55418648642, 69454242114688, 74539841, 454214578213];
        for c in word.bytes() {
            for i in 0..8 {
                result = result.overflowing_mul(c as usize + RANDOM_SEED[i*2]).0;
                result = result.overflowing_add(c as usize + RANDOM_SEED[i*2+1]).0;
            }
        }
        result % (N * 8)
    }
}
