use std::sync::Arc;

use tokio::sync::RwLock;

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    pub id: usize,
    pub title: String,
    pub overview: String,
    pub genres: Vec<String>,
    pub poster: String,
    pub release_date: i64,
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

#[derive(Default)]
pub struct MovieIndexInner<const N: usize> {
    movies: Vec<Movie>,
    filter: Filter<N>,
}

#[derive(Default)]
pub struct MovieIndex<const N: usize> {
    inner: Arc<RwLock<MovieIndexInner<N>>>,
}

#[async_trait]
impl<const N: usize> Store<N> for MovieIndex<N> {
    type SearchResult = Movie;

    fn hash_word(word: &str) -> Vec<usize> {
        let mut result = 1usize;
        const RANDOM_SEED: [usize; 16] = [542587211452, 5242354514, 245421154, 4534542154, 542866467, 545245414, 7867569786914, 88797854597, 24542187316, 645785447, 434963879, 4234274, 55418648642, 69454242114688, 74539841, 454214578213];
        for c in word.bytes() {
            for i in 0..8 {
                result = result.overflowing_mul(c as usize + RANDOM_SEED[i*2]).0;
                result = result.overflowing_add(c as usize + RANDOM_SEED[i*2+1]).0;
            }
        }
        vec![result % (N * 8)]
    }

    async fn get_filter(&self) -> Filter<N> {
        self.inner.read().await.filter.clone()
    }

    fn search(&self, words: Vec<String>, min_matching: usize) -> std::pin::Pin<Box<dyn futures::Future<Output = Vec<Self::SearchResult> > +Send+Sync+'static> >  {
        let inner2 = Arc::clone(&self.inner);
        Box::pin(async move {
            let inner = inner2.read().await;
            let present_words = words.iter().filter(|w| inner.filter.get_word::<Self>(w)).count();
            if present_words < min_matching {
                return Vec::new();
            }

            inner.movies.iter().filter(|movie| {
                let mut matches = 0;
                for word in &words {
                    if movie.words().contains(word) {
                        matches += 1;
                    }
                }
                matches >= min_matching
            }).cloned().collect()
        })
    }
}

impl<const N: usize> MovieIndex<N> {
    pub async fn insert_document(&self, doc: Movie) {
        let mut inner = self.inner.write().await;
        doc.words().iter().for_each(|w| inner.filter.add_word::<Self>(w));
        inner.movies.push(doc);
    }

    pub async fn insert_documents(&self, docs: &[Movie]) {
        let mut inner = self.inner.write().await;
        for doc in docs {
            doc.words().iter().for_each(|w| inner.filter.add_word::<Self>(w));
            inner.movies.push(doc.to_owned());
        }
    }
} 

impl SearchResult for Movie {
    type Cid = usize;

    fn cid(&self) -> Self::Cid {
        self.id
    }

    fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        serde_json::from_slice(bytes).unwrap()
    }
}

pub fn get_movies() -> Vec<Movie> {
    let data = match std::fs::read_to_string("movies.json") {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::process::Command::new("sh")
                .arg("-c")
                .arg("wget https://www.meilisearch.com/movies.json")
                .output()
                .expect("failed to download movies.json");
            std::fs::read_to_string("movies.json").unwrap()
        },
        e => e.unwrap(),
    };

    serde_json::from_str::<Vec<Movie>>(&data).unwrap()
}
