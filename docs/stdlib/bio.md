# bio

The `tt.bio` module provides bioinformatics tools including DNA/RNA/Protein sequence manipulation, genetic code tables, sequence alignment, FASTA format I/O, restriction enzyme analysis, and phylogenetic tree construction.

```titrate
import tt.bio.Sequence;
import tt.bio.CodonTable;
import tt.bio.Alignment;
import tt.bio.FastaReader;
import tt.bio.FastaWriter;
import tt.bio.RestrictionEnzyme;
import tt.bio.PhyloTree;
```

## Sequence

Represents a DNA, RNA, or Protein sequence with standard bioinformatics operations.

- `fn init(s: string, t: string)` — create a sequence with string data and type (`"dna"`, `"rna"`, or `"protein"`)
- `length(): int` — sequence length
- `substring(start: int, end: int): Sequence` — extract a subsequence
- `complement(): Sequence` — compute the complement (A↔T, C↔G, U↔A)
- `reverseComplement(): Sequence` — compute the reverse complement
- `transcribe(): Sequence` — DNA→RNA (T→U) or RNA→DNA (U→T)
- `gcContent(): double` — GC content as a percentage (0–100)
- `mutation(position: int, newBase: string): Sequence` — point mutation at a given position
- `countBase(base: string): int` — count occurrences of a base
- `getType(): string` — return the sequence type
- `toString(): string` — return the raw sequence string

```titrate
let dna = new Sequence("ATGCGATCGA", "dna");
io::println(Integer.toString(String.length(dna.toString())));  // 10
io::println(Double.toString(dna.gcContent()));                   // 60.0

let rna = dna.transcribe();
io::println(rna.toString());            // "AUGCGAUCGA"

let rc = dna.reverseComplement();
io::println(rc.toString());             // "TCGATCGCAT"

let mutated = dna.mutation(2, "A");
io::println(mutated.toString());        // "ATACGATCGA"
```

## CodonTable

Genetic code lookup loaded from `data/bio/codon_tables.json`. Supports the standard table and alternative genetic codes.

- `lookup(codon: string): string` — three-letter amino acid name for a codon (e.g. `"AUG"` → `"Met"`)
- `lookupSingleLetter(codon: string): string` — single-letter amino acid (e.g. `"AUG"` → `"M"`, stop codons return `"*"`)
- `isStartCodon(codon: string): bool` — check if codon is a start codon
- `isStopCodon(codon: string): bool` — check if codon is a stop codon
- `getStartCodons(): ArrayList<string>` — list all start codons
- `getStopCodons(): ArrayList<string>` — list all stop codons
- `getCodonTable(name: string): HashMap<string, string>` — get an alternative codon table by name
- `availableTables(): ArrayList<string>` — list available codon table names
- `translate(dna: string): string` — translate DNA to protein (skipping stop codons)
- `translateWithStops(dna: string): ArrayList<string>` — translate and split at stop codons

```titrate
import tt.bio.CodonTable;

io::println(CodonTable.lookup("AUG"));             // "Met"
io::println(CodonTable.lookupSingleLetter("AUG")); // "M"
io::println(CodonTable.isStartCodon("AUG"));       // true
io::println(CodonTable.isStopCodon("UAA"));        // true

let protein: string = CodonTable.translate("ATGGCGTAA");
io::println(protein);  // "MA"
```

## Alignment

Sequence alignment using Needleman-Wunsch (global) and Smith-Waterman (local) algorithms. Scoring matrices are loaded from `data/bio/scoring_matrices.json` (BLOSUM62 by default).

### AlignmentResult

Result of a sequence alignment.

- `fn init(s1: string, s2: string, sc: int, id: double)` — aligned sequences, score, and identity percentage
- `public string seq1Aligned` — first sequence with gaps
- `public string seq2Aligned` — second sequence with gaps
- `public int score` — alignment score
- `public double identity` — percent identity (0–100)

### Alignment Functions

- `needlemanWunsch(seq1: string, seq2: string, gapPenalty: int): AlignmentResult` — global alignment
- `smithWaterman(seq1: string, seq2: string, gapPenalty: int): AlignmentResult` — local alignment

```titrate
import tt.bio.Alignment;

let result = Alignment.needlemanWunsch("HEAGAWGHEE", "PAWHEAE", -8);
io::println(result.score);               // alignment score
io::println(result.identity);            // percent identity
io::println(result.seq1Aligned);         // aligned sequence 1
io::println(result.seq2Aligned);         // aligned sequence 2

let local = Alignment.smithWaterman("HEAGAWGHEE", "PAWHEAE", -8);
io::println(local.score);                // local alignment score
```

## FastaReader

FASTA format file parsing with sequence iteration and multi-sequence support.

### FastaRecord

A single FASTA record with header and sequence.

- `fn init(h: string, s: string)` — create a record with header and sequence
- `id(): string` — extract the ID (text before first space in header)
- `description(): string` — extract the description (text after first space)
- `public string header` — full header line
- `public string sequence` — sequence data

### FastaReader Functions

- `readFasta(path: string): ArrayList<FastaRecord>` — read a FASTA file, returns all records

```titrate
import tt.bio.FastaReader;

let records = FastaReader.readFasta("sequences.fasta");
var i: int = 0;
while (i < records.size()) {
    let rec = records.get(i);
    io::println(rec.id());
    io::println(rec.sequence);
    i = i + 1;
}
```

## FastaWriter

FASTA format file writing with configurable line wrapping.

- `writeFasta(path: string, records: ArrayList<FastaRecord>, lineWidth: int): void` — write records to a FASTA file
- `formatFasta(header: string, sequence: string, lineWidth: int): string` — format a single sequence as FASTA string

```titrate
import tt.bio.FastaWriter;

let records = new ArrayList<FastaRecord>();
records.add(new FastaRecord("seq1 Some description", "ATGCGATCGA"));
records.add(new FastaRecord("seq2 Another sequence", "GCTAGCTAGC"));
FastaWriter.writeFasta("output.fasta", records, 80);

let formatted: string = FastaWriter.formatFasta("seq1", "ATGCGATCGA", 5);
io::println(formatted);
// >seq1
// ATGCG
// ATCGA
```

## RestrictionEnzyme

Restriction enzyme database loaded from `data/bio/restriction_enzymes.json`, with cut site recognition and digest simulation.

### RestrictionEnzyme Class

- `fn init(n: string, site: string, ct: int, cb: int, t: string)` — create enzyme with name, recognition site, cut positions, and type
- `public string name` — enzyme name
- `public string recognitionSite` — recognition sequence (supports IUPAC ambiguity codes)
- `public int cutTop` — top-strand cut offset
- `public int cutBottom` — bottom-strand cut offset
- `public string enzymeType` — enzyme type

### RestrictionEnzyme Functions

- `getEnzyme(name: string): RestrictionEnzyme` — look up enzyme by name
- `availableEnzymes(): ArrayList<string>` — list all available enzyme names
- `findCutSites(dna: string, enzymeName: string): ArrayList<int>` — find all cut positions for an enzyme
- `digest(dna: string, enzymeName: string): ArrayList<string>` — simulate restriction digest, returns DNA fragments

```titrate
import tt.bio.RestrictionEnzyme;

let enzyme = RestrictionEnzyme.getEnzyme("EcoRI");
io::println(enzyme.recognitionSite);  // "GAATTC"

let sites = RestrictionEnzyme.findCutSites("GAATTCGAATTC", "EcoRI");
io::println(Integer.toString(sites.size()));  // 2

let fragments = RestrictionEnzyme.digest("GAATTCGAATTC", "EcoRI");
var i: int = 0;
while (i < fragments.size()) {
    io::println(fragments.get(i));
    i = i + 1;
}
```

## PhyloTree

Phylogenetic tree construction with UPGMA and neighbor-joining algorithms, plus Newick format I/O.

### PhyloNode

A node in a phylogenetic tree.

- `fn init(n: string, d: double)` — create node with name and branch length
- `isLeaf(): bool` — check if node is a leaf
- `addChild(child: PhyloNode): void` — add a child node
- `leafCount(): int` — count descendant leaves
- `public string name` — node label
- `public double distance` — branch length to parent
- `public ArrayList<PhyloNode> children` — child nodes
- `public PhyloNode parent` — parent node

### PhyloTree Class

- `fn init(r: PhyloNode)` — create tree from root node
- `getLeaves(): ArrayList<PhyloNode>` — collect all leaf nodes
- `toNewick(): string` — serialize tree to Newick format

### PhyloTree Functions

- `parseNewick(s: string): PhyloTree` — parse a Newick-format string into a tree
- `upgma(distanceMatrix: ArrayList<ArrayList<double>>, names: ArrayList<string>): PhyloTree` — construct tree using UPGMA
- `neighborJoining(distanceMatrix: ArrayList<ArrayList<double>>, names: ArrayList<string>): PhyloTree` — construct tree using neighbor-joining

```titrate
import tt.bio.PhyloTree;

// Build a tree manually
let root = new PhyloNode("", 0.0);
let a = new PhyloNode("A", 1.0);
let b = new PhyloNode("B", 2.0);
root.addChild(a);
root.addChild(b);
let tree = new PhyloTree(root);
io::println(tree.toNewick());  // "(A:1.0,B:2.0);

// Parse Newick
let parsed = PhyloTree.parseNewick("((A:1.0,B:1.0):0.5,C:1.5);");

// UPGMA from distance matrix
let dist = new ArrayList<ArrayList<double>>();
// ... populate distance matrix ...
let names = new ArrayList<string>();
names.add("A");
names.add("B");
names.add("C");
let upgmaTree = PhyloTree.upgma(dist, names);
io::println(upgmaTree.toNewick());
```
