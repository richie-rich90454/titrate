# Bioinformatics with Titrate

Titrate's `tt.bio` module provides a comprehensive toolkit for bioinformatics — from sequence analysis and alignment to phylogenetics and restriction enzyme analysis. This guide covers the core functionality and walks through a complete DNA analysis example.

## Sequence Analysis

The `Sequence` class represents biological sequences (DNA, RNA, protein) and provides methods for common operations like complement, transcription, translation, and composition analysis.

```titrate
import tt::bio::Sequence;
```

### Creating Sequences

```titrate
// DNA sequence
let dna = Sequence.dna("ATGCGATCGAATTCGCTAG");

// RNA sequence
let rna = Sequence.rna("AUGCGAU CGAAUUCGCUAG");

// Protein sequence
let protein = Sequence.protein("MKTAYIAKQRQISFVKSH");

// Validate sequence against its alphabet
if (dna.isValid()) {
    io::println("Valid DNA sequence");
}
```

### Complement and Reverse Complement

```titrate
let dna = Sequence.dna("ATGCGATCGAATTCGCTAG");

// Complement (A↔T, C↔G)
let comp = dna.complement();
io::println("Complement: " + comp.toString());  // TACGCTAGCTTAAGCGATC

// Reverse complement (standard for reading the opposite strand)
let revComp = dna.reverseComplement();
io::println("RevComp: " + revComp.toString());  // CTAGCGAATTCGATCGCAT
```

### Transcription (DNA → RNA)

```titrate
let dna = Sequence.dna("ATGCGATCGAATTCGCTAG");
let rna = dna.transcribe();
io::println("RNA: " + rna.toString());  // AUGCGAU CGAAUUCGCUAG
```

### Translation (RNA → Protein)

```titrate
let rna = Sequence.rna("AUGCGAU CGAAUUCGCUAG");
let protein = rna.translate();
io::println("Protein: " + protein.toString());

// Translate with a specific codon table
let protein2 = rna.translateWithTable(1);  // Standard genetic code (table 1)
```

### GC Content

```titrate
let dna = Sequence.dna("ATGCGATCGAATTCGCTAG");
let gc = dna.gcContent();
io::println("GC content: " + Double.toString(gc * 100.0) + "%");
```

### Sequence Composition

```titrate
// Count individual bases
let counts = dna.baseCounts();
io::println("A: " + Integer.toString(counts.get("A")));
io::println("T: " + Integer.toString(counts.get("T")));
io::println("G: " + Integer.toString(counts.get("G")));
io::println("C: " + Integer.toString(counts.get("C")));

// Sequence length
io::println("Length: " + Integer.toString(String.length(dna)));
```

## Sequence Alignment

The `tt.bio` module implements both global and local sequence alignment algorithms with configurable scoring.

```titrate
import tt::bio::Alignment;
```

### Needleman-Wunsch Global Alignment

Global alignment finds the best alignment across the entire length of both sequences:

```titrate
let seq1 = Sequence.dna("GCATGCU");
let seq2 = Sequence.dna("GATTACA");

// Default scoring: match=+1, mismatch=-1, gap=-1
let result = Alignment.needlemanWunsch(seq1, seq2);

io::println("Score: " + Integer.toString(result.score()));
io::println("Alignment:");
io::println("  " + result.alignedA());  // GCATG-CU
io::println("  " + result.alignedB());  // G-ATTACA

// Custom scoring
let customResult = Alignment.needlemanWunsch(seq1, seq2, 2, -1, -2);
//                                              match, mismatch, gap
```

### Smith-Waterman Local Alignment

Local alignment finds the best matching subsequences — useful when sequences have regions of high similarity flanked by divergent regions:

```titrate
let seq1 = Sequence.dna("TGTTACGG");
let seq2 = Sequence.dna("GGTTGACTA");

let result = Alignment.smithWaterman(seq1, seq2);

io::println("Score: " + Integer.toString(result.score()));
io::println("Local alignment:");
io::println("  " + result.alignedA());
io::println("  " + result.alignedB());
```

### Scoring Matrices

For protein alignment, substitution matrices capture the likelihood of amino acid substitutions:

```titrate
// BLOSUM62 — standard for protein alignment
let blosum62 = ScoringMatrix.blosum62();

// BLOSUM45 — better for distant homologs
let blosum45 = ScoringMatrix.blosum45();

// PAM250 — alternative substitution matrix
let pam250 = ScoringMatrix.pam250();

// Use a scoring matrix in alignment
let prot1 = Sequence.protein("HEAGAWGHEE");
let prot2 = Sequence.protein("PAWHEAE");
let result = Alignment.needlemanWunsch(prot1, prot2, blosum62, -8);
//                                              matrix, gap penalty
```

## Codon Tables

The `CodonTable` class provides the standard genetic code and alternative codon tables used by different organisms and organelles.

```titrate
import tt::bio::CodonTable;
```

### Standard Genetic Code

```titrate
// Table 1: Standard genetic code
let table = CodonTable.getTable(1);

// Look up a codon
let aa = table.lookup("AUG");  // "M" (Methionine)
io::println("AUG encodes: " + aa);

// Check all codons
let codons = table.codons();
for (codon in codons) {
    io::println(codon + " → " + table.lookup(codon));
}
```

### Start and Stop Codons

```titrate
let table = CodonTable.getTable(1);

// Start codons
let starts = table.startCodons();
io::println("Start codons:");
for (c in starts) {
    io::println("  " + c);  // ATG (standard)
}

// Stop codons
let stops = table.stopCodons();
io::println("Stop codons:");
for (c in stops) {
    io::println("  " + c);  // TAA, TAG, TGA
}

// Check if a codon is a start or stop
if (table.isStart("ATG")) {
    io::println("ATG is a start codon");
}
if (table.isStop("TAA")) {
    io::println("TAA is a stop codon");
}
```

### Alternative Codon Tables

```titrate
// Table 2: Vertebrate mitochondrial code
let mitoTable = CodonTable.getTable(2);

// Table 5: Invertebrate mitochondrial code
let invertMitoTable = CodonTable.getTable(5);

// Table 11: Bacterial, archaeal, and plant plastid code
let bacterialTable = CodonTable.getTable(11);

// List all available tables
let tables = CodonTable.availableTables();
for (t in tables) {
    io::println("Table " + Integer.toString(t.id) + ": " + t.name);
}
```

## FASTA I/O

The `FastaReader` and `FastaWriter` classes handle reading and writing FASTA format files — the standard format for sequence data.

```titrate
import tt::bio::FastaReader;
import tt::bio::FastaWriter;
import tt::bio::FastaRecord;
```

### Reading FASTA Files

```titrate
// Read a single-sequence FASTA file
let records = FastaReader.readFasta("sequence.fasta");
if (records.size() > 0) {
    let record = records.get(0);
    io::println("ID: " + record.id());
    io::println("Description: " + record.description());
    io::println("Sequence: " + record.sequence.toString());
}

// Read a multi-sequence FASTA file
let allRecords = FastaReader.readFasta("sequences.fasta");
io::println("Read " + Integer.toString(allRecords.size()) + " sequences");

for (record in allRecords) {
    io::println(">" + record.id() + " " + record.description());
    io::println(record.sequence.toString());
}
```

### Writing FASTA Files

```titrate
// Create a FASTA record
let rec1 = new FastaRecord("seq001", "Example sequence ATGCGATCGA");

// Write a single record (create a list with one record)
let singleList = new ArrayList<FastaRecord>();
singleList.add(rec1);
FastaWriter.writeFastaDefault("output.fasta", singleList);

// Write multiple records
let records = new ArrayList<FastaRecord>();
records.add(rec1);
records.add(new FastaRecord("seq002", "Another sequence TTAGCGCTA"));
FastaWriter.writeFasta("multi_output.fasta", records, 60);
```

### Multi-Sequence Handling

```titrate
// Filter sequences by length
let longSeqs = new ArrayList<FastaRecord>();
for (record in records) {
    if (String.length(record.sequence.toString()) > 100) {
        longSeqs.add(record);
    }
}

// Compute GC content for all sequences
for (record in records) {
    let gc = record.sequence.gcContent();
    io::println(record.id() + ": GC=" + Double.toString(gc * 100.0) + "%");
}
```

## Restriction Enzyme Analysis

The `RestrictionEnzyme` module provides a database of common restriction enzymes and tools for finding cut sites and simulating digests.

```titrate
import tt::bio::RestrictionEnzyme;
```

### Enzyme Database

```titrate
// Look up an enzyme by name
let ecori = RestrictionEnzyme.getEnzyme("EcoRI");
io::println("Enzyme: " + ecori.name);
io::println("Recognition site: " + ecori.site);       // GAATTC
io::println("Cut position (top): " + Integer.toString(ecori.cutTop));     // 1 (after G)
io::println("Cut position (bottom): " + Integer.toString(ecori.cutBottom)); // 5 (after A on complement)

// List all available enzymes
let allEnzymes = RestrictionEnzyme.availableEnzymes();
io::println("Available enzymes: " + Integer.toString(allEnzymes.size()));
```

### Cut Site Recognition

```titrate
let dna = Sequence.dna("AAGAATTCTGAAGCATGCGATCGAATTCGCTAG");

// Find all EcoRI cut sites using the standalone function
let sites = RestrictionEnzyme.findCutSites(dna, "EcoRI");
io::println("EcoRI cut sites:");
for (site in sites) {
    io::println("  Position " + Integer.toString(site));
}
```

### Digest Simulation

```titrate
// Simulate a single-enzyme digest
let fragments = RestrictionEnzyme.digest(dna, "EcoRI");
io::println("Fragments after EcoRI digest:");
for (fragment in fragments) {
    io::println("  Length: " + Integer.toString(String.length(fragment.toString())) +
                " — " + fragment.toString());
}

// Simulate a double digest (two enzymes)
let doubleFragments = RestrictionEnzyme.multiDigest(dna, ["EcoRI", "BamHI"]);
io::println("Double digest fragments: " + Integer.toString(doubleFragments.size()));
```

## Phylogenetics

The `tt.bio` module provides algorithms for constructing phylogenetic trees from distance matrices.

```titrate
import tt::bio::PhyloTree;
```

### UPGMA

UPGMA (Unweighted Pair Group Method with Arithmetic Mean) produces ultrametric trees assuming a constant molecular clock:

```titrate
// Build distance matrix as 2D ArrayList
let dm = new ArrayList<ArrayList<double>>();
let names = new ArrayList<string>();
names.add("A"); names.add("B"); names.add("C"); names.add("D");

let row0 = new ArrayList<double>(); row0.add(0.0); row0.add(2.0); row0.add(4.0); row0.add(6.0);
let row1 = new ArrayList<double>(); row1.add(2.0); row1.add(0.0); row1.add(4.0); row1.add(6.0);
let row2 = new ArrayList<double>(); row2.add(4.0); row2.add(4.0); row2.add(0.0); row2.add(4.0);
let row3 = new ArrayList<double>(); row3.add(6.0); row3.add(6.0); row3.add(4.0); row3.add(0.0);
dm.add(row0); dm.add(row1); dm.add(row2); dm.add(row3);

// Build tree
let tree = PhyloTree.upgma(dm, names);
io::println("UPGMA tree (Newick): " + tree.toNewick());
```

### Neighbor-Joining

Neighbor-joining does not assume a molecular clock and is more appropriate when evolutionary rates vary:

```titrate
let tree = PhyloTree.neighborJoining(dm, names);
io::println("NJ tree (Newick): " + tree.toNewick());
```

### Newick Format

```titrate
// Parse a Newick string
let parsed = PhyloTree.parseNewick("((A:1,B:1):2,(C:3,D:3):1);");

// Convert back to Newick
io::println(parsed.toNewick());

// Access tree properties
let leaves = parsed.getLeaves();
io::println("Leaves: " + Integer.toString(leaves.size()));

let totalHeight = parsed.depth();
io::println("Tree depth: " + Double.toString(totalHeight));
```

## End-to-End Example: Analyzing a DNA Sequence for ORFs and Restriction Sites

This example takes a DNA sequence, finds all open reading frames (ORFs), identifies restriction enzyme cut sites, and reports the results.

```titrate
import tt::bio::Sequence;
import tt::bio::CodonTable;
import tt::bio::RestrictionEnzyme;
import tt::bio::FastaWriter;
import tt::bio::FastaRecord;

public class ORFResult {
    public int start;
    public int end;
    public string protein;
    public int frame;

    public fn init(s: int, e: int, prot: string, f: int) {
        this.start = s;
        this.end = e;
        this.protein = prot;
        this.frame = f;
    }
}

public fn findORFs(dna: Sequence, minLen: int): ArrayList<ORFResult> {
    let table = CodonTable.getTable(1);
    let results = new ArrayList<ORFResult>();
    let seq = dna.toString();
    let len = String.length(seq);

    // Search all three reading frames
    for (frame in 0..3) {
        var i = frame;
        while (i + 2 < len) {
            let codon = String.substring(seq, i, i + 3);

            if (table.isStart(codon)) {
                var pos = i + 3;
                var found = false;
                while (pos + 2 < len) {
                    let nextCodon = String.substring(seq, pos, pos + 3);
                    if (table.isStop(nextCodon)) {
                        let orfLen = pos + 3 - i;
                        if (orfLen >= minLen) {
                            let orfSeq = Sequence.dna(String.substring(seq, i, pos + 3));
                            let protein = orfSeq.transcribe().translateWithTable(1);
                            results.add(new ORFResult(i, pos + 3, protein.toString(), frame + 1));
                        }
                        found = true;
                        break;
                    }
                    pos = pos + 3;
                }
                if (found) {
                    i = pos + 3;
                } else {
                    i = i + 3;
                }
            } else {
                i = i + 3;
            }
        }
    }

    return results;
}

public fn findRestrictionSites(dna: Sequence, enzymeNames: ArrayList<string>): void {
    for (name in enzymeNames) {
        let sites = RestrictionEnzyme.findCutSites(dna, name);
        if (sites.size() > 0) {
            io::println("  " + name + ": " +
                        Integer.toString(sites.size()) + " site(s)");
            for (site in sites) {
                io::println("    Position: " + Integer.toString(site));
            }
        }
    }
}

public fn main(): void {
    // Load a DNA sequence
    let dna = Sequence.dna(
        "ATGCGATCGAATTCGCTAGATGAAAGCTGGCATGCTAGGAATTCGATG" +
        "CCGATCGATCGAATTCGGATCCATGAAAAGCTTTGATAGCGATCGAAT" +
        "TCGCTAGATGAAAGCTGGCATGCTAGGAATTCGATGCCGATCGATCGA"
    );

    io::println("=== DNA Sequence Analysis ===");
    io::println("Length: " + Integer.toString(String.length(dna.toString())) + " bp");
    io::println("GC content: " + Double.toString(dna.gcContent() * 100.0) + "%");
    io::println("");

    // Find ORFs (minimum 30 nucleotides = 10 amino acids)
    io::println("--- Open Reading Frames (min 30 nt) ---");
    let orfs = findORFs(dna, 30);
    for (orf in orfs) {
        io::println("  Frame " + Integer.toString(orf.frame) +
                    ": pos " + Integer.toString(orf.start) +
                    "-" + Integer.toString(orf.end) +
                    " (" + Integer.toString(orf.end - orf.start) + " nt)" +
                    " → " + orf.protein);
    }
    io::println("");

    // Find restriction sites
    io::println("--- Restriction Enzyme Sites ---");
    let enzymes = new ArrayList<string>();
    enzymes.add("EcoRI");
    enzymes.add("BamHI");
    enzymes.add("HindIII");
    enzymes.add("NotI");
    findRestrictionSites(dna, enzymes);
    io::println("");

    // Simulate EcoRI digest
    io::println("--- EcoRI Digest ---");
    let fragments = RestrictionEnzyme.digest(dna, "EcoRI");
    for (i in 0..fragments.size()) {
        let frag = fragments.get(i);
        io::println("  Fragment " + Integer.toString(i + 1) +
                    ": " + Integer.toString(frag.length()) + " bp");
    }

    // Save the sequence to FASTA
    let recList = new ArrayList<FastaRecord>();
    recList.add(new FastaRecord("analyzed_seq", "Sample DNA sequence"));
    FastaWriter.writeFastaDefault("analyzed.fasta", recList);
    io::println("");
    io::println("Sequence saved to analyzed.fasta");
}
```

::: tip Working with real data
When working with real genomic data, always validate your input sequences with `isValid()` before analysis. Sequences from public databases may contain ambiguous bases (N, R, Y, etc.) that need special handling.
:::

## What's Next?

- [Scientific Computing](./scientific-computing) — NDArray and Matrix for numerical work
- [File I/O](./file-io) — reading and writing files
- [Standard Library](./stdlib) — full module reference
