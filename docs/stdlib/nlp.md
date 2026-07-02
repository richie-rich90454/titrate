# nlp

The `tt.nlp` module provides natural language processing tools including tokenization, stemming, text vectorization, string distance metrics, and text classification.

```titrate
import tt.nlp.Tokenizer;
import tt.nlp.Stemmer;
import tt.nlp.Vectorize;
import tt.nlp.Distance;
import tt.nlp.Classifier;
```

## Tokenizer

Text tokenization with multiple strategies, punctuation handling, and stop word filtering.

- `fn init()` — create a tokenizer with default settings

**Word tokenizer:**
- `tokenize(text: string): ArrayList<string>` — tokenize text into words (handles contractions, punctuation)
- `tokenizeWords(text: string): ArrayList<string>` — simple word tokenizer (split on whitespace and punctuation)

**Sentence tokenizer:**
- `tokenizeSentences(text: string): ArrayList<string>` — split text into sentences (handles abbreviations, decimal points)

**Regex tokenizer:**
- `tokenizeRegex(text: string, pattern: string): ArrayList<string>` — tokenize using a regex pattern

**Whitespace tokenizer:**
- `tokenizeWhitespace(text: string): ArrayList<string>` — split on whitespace only

**Punctuation handling:**
- `removePunctuation(tokens: ArrayList<string>): ArrayList<string>` — strip punctuation from all tokens
- `keepPunctuation(tokens: ArrayList<string>): ArrayList<string>` — keep only tokens containing punctuation

**Stop words:**
- `removeStopWords(tokens: ArrayList<string>): ArrayList<string>` — remove common stop words (loaded from `data/nlp/stop_words.json`)
- `isStopWord(word: string): bool` — check if a word is a stop word
- `loadStopWords(path: string): void` — load custom stop word list from JSON file
- `stopWords(): ArrayList<string>` — current stop word list

**Utility:**
- `toLowerCase(tokens: ArrayList<string>): ArrayList<string>` — convert all tokens to lowercase
- `ngrams(tokens: ArrayList<string>, n: int): ArrayList<string>` — generate n-grams from token list

```titrate
let tok = new Tokenizer();
let words = tok.tokenize("Hello, world! This is a test.");
// ["Hello", ",", "world", "!", "This", "is", "a", "test", "."]

let sentences = tok.tokenizeSentences("Dr. Smith went home. He was tired.");
// ["Dr. Smith went home.", "He was tired."]

let filtered = tok.removeStopWords(tok.toLowerCase(words));
let bigrams = tok.ngrams(filtered, 2);
```

## Stemmer

Word stemming and lemmatization with multiple algorithm implementations.

- `fn init()` — create a stemmer with default algorithm (Porter)

**Porter stemmer:**
- `stem(word: string): string` — stem a single word using the Porter algorithm
- `stemAll(words: ArrayList<string>): ArrayList<string>` — stem all words in a list
- `Stemmer.porter(): Stemmer` — create a Porter stemmer instance

**Snowball stemmer:**
- `Stemmer.snowball(language: string): Stemmer` — create a Snowball stemmer for a given language
- `stem(word: string): string` — stem using Snowball algorithm

**Lancaster stemmer:**
- `Stemmer.lancaster(): Stemmer` — create a Lancaster (Paice/Husk) stemmer
- `stem(word: string): string` — aggressive stemming via Lancaster rules

**Lemmatization:**
- `lemmatize(word: string): string` — reduce word to dictionary form using lemmatization rules
- `lemmatizeAll(words: ArrayList<string>): ArrayList<string>` — lemmatize all words in a list

```titrate
let porter = Stemmer.porter();
io::println(porter.stem("running"));    // "run"
io::println(porter.stem("caresses"));   // "caress"
io::println(porter.stem("ponies"));     // "poni"

let snowball = Stemmer.snowball("english");
io::println(snowball.stem("generalization"));  // "general"

let lancaster = Stemmer.lancaster();
io::println(lancaster.stem("maximum"));  // "maxim"

let lemma = porter.lemmatize("better");
io::println(lemma);  // "good" (via lemmatization rules)
```

## Vectorize

Text vectorization for converting documents into numerical representations: bag-of-words, TF-IDF, and hashing vectorizer.

- `fn init()` — create a vectorizer with default settings

**Bag-of-words:**
- `Vectorize.bow(): Vectorize` — create a bag-of-words vectorizer
- `fit(documents: ArrayList<string>): void` — build vocabulary from training documents
- `transform(document: string): ArrayList<double>` — convert document to BoW vector
- `fitTransform(documents: ArrayList<string>): ArrayList<ArrayList<double>>` — fit and transform in one step

**TF-IDF vectorizer:**
- `Vectorize.tfidf(): Vectorize` — create a TF-IDF vectorizer
- `fit(documents: ArrayList<string>): void` — compute IDF weights from training documents
- `transform(document: string): ArrayList<double>` — convert document to TF-IDF vector
- `fitTransform(documents: ArrayList<string>): ArrayList<ArrayList<double>>` — fit and transform in one step

**Hashing vectorizer:**
- `Vectorize.hashing(numFeatures: int): Vectorize` — create a hashing vectorizer with fixed feature count
- `transform(document: string): ArrayList<double>` — convert document to hashed feature vector

**Vocabulary management:**
- `vocabulary(): HashMap<string, int>` — term-to-index mapping
- `vocabularySize(): int` — number of terms in vocabulary
- `termFrequency(document: string): HashMap<string, int>` — raw term counts for a document
- `documentFrequency(): HashMap<string, int>` — number of documents containing each term
- `idfWeights(): ArrayList<double>` — IDF weight vector

**Document-term matrix:**
- `documentTermMatrix(documents: ArrayList<string>): ArrayList<ArrayList<double>>` — build full document-term matrix

```titrate
let docs = new ArrayList<string>();
docs.add("the cat sat on the mat");
docs.add("the dog sat on the log");
docs.add("the cat and the dog played");

let tfidf = Vectorize.tfidf();
let matrix = tfidf.fitTransform(docs);
let vocab = tfidf.vocabulary();
io::println("Vocabulary size: " + Integer.toString(tfidf.vocabularySize()));

let query = tfidf.transform("the cat sat");
// TF-IDF vector for the query document
```

## Distance

String distance and phonetic similarity metrics for comparing text.

- `fn init()` — create a distance calculator

**Edit distances:**
- `levenshtein(a: string, b: string): int` — Levenshtein edit distance (insertions, deletions, substitutions)
- `levenshteinRatio(a: string, b: string): double` — normalized Levenshtein distance in [0, 1]
- `damerauLevenshtein(a: string, b: string): int` — Damerau-Levenshtein distance (includes transpositions)
- `damerauLevenshteinRatio(a: string, b: string): double` — normalized Damerau-Levenshtein distance

**Similarity metrics:**
- `jaroWinkler(a: string, b: string): double` — Jaro-Winkler similarity in [0, 1] (1 = identical)
- `jaro(a: string, b: string): double` — Jaro similarity in [0, 1]

**Phonetic algorithms:**
- `soundex(word: string): string` — Soundex phonetic code (four-character code)
- `metaphone(word: string): string` — Metaphone phonetic key
- `doubleMetaphone(word: string): (string, string)` — Double Metaphone (primary and alternate keys)

**Utility:**
- `hamming(a: string, b: string): int` — Hamming distance (equal-length strings only)
- `ngramSimilarity(a: string, b: string, n: int): double` — n-gram overlap similarity in [0, 1]

```titrate
let dist = new Distance();
io::println(Integer.toString(dist.levenshtein("kitten", "sitting")));  // 3
io::println(Double.toString(dist.jaroWinkler("MARTHA", "MARHTA")));    // ≈ 0.961
io::println(dist.soundex("Robert"));    // "R163"
io::println(dist.metaphone("Knight"));  // "NFT"

let (primary, alternate) = dist.doubleMetaphone("Catherine");
io::println("Primary: " + primary + ", Alternate: " + alternate);
```

## Classifier

Text classification with Naive Bayes, sentiment analysis, and evaluation metrics.

- `fn init()` — create a Naive Bayes text classifier

**Training:**
- `train(documents: ArrayList<string>, labels: ArrayList<string>): void` — train classifier on labeled documents
- `addDocument(text: string, label: string): void` — add a single labeled document incrementally

**Prediction:**
- `predict(text: string): string` — classify a document, returns predicted label
- `predictProba(text: string): HashMap<string, double>` — class probability estimates
- `predictAll(documents: ArrayList<string>): ArrayList<string>` — classify multiple documents

**Sentiment analysis:**
- `sentiment(text: string): double` — sentiment score from -1.0 (negative) to 1.0 (positive) using lexicon from `data/nlp/sentiment_lexicon.json`
- `loadSentimentLexicon(path: string): void` — load custom sentiment lexicon from JSON file

**Accuracy metrics:**
- `accuracy(predicted: ArrayList<string>, actual: ArrayList<string>): double` — classification accuracy
- `precision(predicted: ArrayList<string>, actual: ArrayList<string>, positiveLabel: string): double` — precision for a given class
- `recall(predicted: ArrayList<string>, actual: ArrayList<string>, positiveLabel: string): double` — recall for a given class
- `f1Score(predicted: ArrayList<string>, actual: ArrayList<string>, positiveLabel: string): double` — F1 score (harmonic mean of precision and recall)
- `confusionMatrix(predicted: ArrayList<string>, actual: ArrayList<string>): HashMap<string, HashMap<string, int>>` — confusion matrix

**Model info:**
- `classes(): ArrayList<string>` — list of class labels
- `classPrior(label: string): double` — prior probability for a class
- `vocabularySize(): int` — number of terms in model vocabulary

```titrate
let clf = new Classifier();

let docs = new ArrayList<string>();
docs.add("great movie loved it");
docs.add("terrible film waste of time");
docs.add("absolutely wonderful performance");
docs.add("boring and dull");

let labels = new ArrayList<string>();
labels.add("positive");
labels.add("negative");
labels.add("positive");
labels.add("negative");

clf.train(docs, labels);

let prediction = clf.predict("amazing acting and great story");
io::println("Predicted: " + prediction);  // "positive"

let proba = clf.predictProba("it was okay");
let sentiment = clf.sentiment("this is wonderful and amazing");
io::println("Sentiment: " + Double.toString(sentiment));  // positive score

let preds = clf.predictAll(docs);
let acc = clf.accuracy(preds, labels);
io::println("Accuracy: " + Double.toString(acc));
```
