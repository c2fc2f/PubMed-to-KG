# PubMed-MeSH-to-KG

A command-line tool written in Rust that downloads the full [PubMed baseline dataset](https://pubmed.ncbi.nlm.nih.gov/) and the [NLM MeSH vocabulary](https://www.nlm.nih.gov/mesh/meshhome.html), then converts them into a set of CSV files ready for bulk import into a [Neo4j](https://neo4j.com/) knowledge graph.

## Overview

PubMed distributes its baseline as hundreds of gzip-compressed XML files. MeSH is published as three large XML files (descriptors, qualifiers, supplemental records). This tool streams both datasets — without loading them fully into memory — and writes Neo4j-compatible CSV node and relationship files.

The project is organized as a Cargo workspace with two independent library crates:

- **`crates/mesh`** — streaming Rust client for the NLM MeSH XML dataset
- **`crates/pubmed`** — asynchronous Rust client for the PubMed baseline

The `pm2kg` binary ties these two crates together and writes the final CSV output.

## Requirements

- Rust toolchain (edition 2024, stable)
- A network connection to reach the NCBI and NLM servers (or a local mirror)
- Sufficient disk space for the downloaded cache and output CSVs (the full PubMed baseline is tens of gigabytes)

## Installation

### From source

```bash
git clone https://github.com/c2fc2f/PubMed-MeSH-to-KG
cd PubMed-MeSH-to-KG
cargo build --release
```

The compiled binary will be at `target/release/pm2kg`.

### With Nix

A Nix flake is provided:

```bash
nix run github:c2fc2f/PubMed-MeSH-to-KG -- --help
# or
nix build
# or, to enter a development shell:
nix develop
```

## Usage

```
pm2kg [OPTIONS]
```

| Flag | Short | Description | Default |
|---|---|---|---|
| `--parallel <N>` | `-p` | Number of concurrent processing tasks | Number of logical CPUs |
| `--no-cache` | `-n` | Disable on-disk caching; stream data directly | caching enabled |
| `--output <DIR>` | `-o` | Directory where CSV files are written | `.` (current directory) |

### Examples

Process all chunks with default parallelism, writing CSVs to `./out`:

```bash
pm2kg --output ./out
```

Run without a cache, using 8 parallel tasks:

```bash
pm2kg --output ./out --no-cache --parallel 8
```

## Caching

By default, downloaded XML files are cached to disk and reused on subsequent runs. The cache location follows the OS convention:

| Platform | Path |
|---|---|
| Linux | `~/.cache/pm2kg/` |
| macOS | `~/Library/Caches/pm2kg/` |
| Windows | `%LOCALAPPDATA%\pm2kg\` |

Pass `--no-cache` to disable caching entirely and stream data directly into the parser.

## Output: Knowledge Graph Schema

All output files are written to the directory specified by `--output`. Node files and relationship files are formatted for [Neo4j's bulk CSV importer](https://neo4j.com/docs/operations-manual/current/tools/neo4j-admin/neo4j-admin-import/).

### Nodes

| File | Label | Description |
|---|---|---|
| `PubMedArticle.csv` | `PubMedArticle` (PubMed) | PubMed articles (PMID, title, …) |
| `PubMedPerson.csv` | `PubMedPerson` (PubMed, PubMedAgent) | Individual authors |
| `PubMedCollective.csv` | `PubMedCollective` (PubMed, PubMedAgent) | Corporate or collective authors |
| `PubMedJournal.csv` | `PubMedJournal` (PubMed) | Journals in which articles were published |
| `PubMedKeyword.csv` | `PubMedKeyword` (PubMed) | Author or MeSH-supplied keywords |
| `MeSHDescriptorQualified.csv` | `MeSHDescriptorQualified` (MeSH) | Junction nodes linking a MeSH heading to its qualifier on a given article |
| `MeSHDescriptor.csv` | `MeSHDescriptor` (MeSH) | MeSH descriptor records |
| `MeSHQualifier.csv` | `MeSHQualifier` (MeSH) | MeSH qualifier records |
| `MeSHSupplemental.csv` | `MeSHSupplemental` (MeSH) | MeSH supplemental concept records |
| `MeSHConcept.csv` | `MeSHConcept` (MeSH) | MeSH concept records |

### Relationships

| File | Type | From → To | Description |
|---|---|---|---|
| `HAS_AUTHOR.csv` | `HAS_AUTHOR` | PubMedArticle → PubMedAgent | Links an article to its authors |
| `IS_PART_OF.csv` | `IS_PART_OF` | PubMedArticle → PubMedJournal | Links an article to its journal |
| `HAS_KEYWORD.csv` | `HAS_KEYWORD` | PubMedArticle → PubMedKeyword | Links an article to its keywords |
| `CITES.csv` | `CITES` | PubMedArticle → PubMedArticle | Citation links between articles |
| `HAS_MESH.csv` | `HAS_MESH` | PubMedArticle → MeSHDescriptorQualified | Links an article to its MeSH headings |
| `HAS_SUPPLEMENTARY_MESH.csv` | `HAS_SUPPLEMENTARY_MESH` | PubMedArticle → MeSHSupplemental | Links an article to supplemental MeSH concepts |
| `HAS_DESCRIPTOR.csv` | `HAS_DESCRIPTOR` | MeSHDescriptorQualified → MeSHDescriptor | Links a qualified heading to its descriptor |
| `HAS_QUALIFIER.csv` | `HAS_QUALIFIER` | MeSHDescriptorQualified → MeSHQualifier | Links a qualified heading to its qualifier |
| `NARROWER_THAN.csv` | `NARROWER_THAN` | MeSH → MeSH | Hierarchical narrower relation in MeSH tree |
| `BROADER_THAN.csv` | `BROADER_THAN` | MeSH → MeSH | Hierarchical broader relation in MeSH tree |
| `RELATED_TO.csv` | `RELATED_TO` | MeSH → MeSH | Related concepts in the MeSH vocabulary |
| `HAS_PHARMACOLOGICAL_ACTION.csv` | `HAS_PHARMACOLOGICAL_ACTION` | MeSH → MeSHDescriptor | Pharmacological action associations |
| `HAS_CONCEPT.csv` | `HAS_CONCEPT` | MeSH → MeSHConcept | Links a record to its constituent concepts |
| `MAPPED_TO.csv` | `MAPPED_TO` | MeSH → MeSH | Supplemental-to-descriptor mappings |

## Importing into Neo4j
 
Once `pm2kg` has finished writing the CSV files, use `neo4j-admin database import full` to bulk-load them into Neo4j. The command below assumes all CSV files are in the current directory and targets the default `neo4j` database.
 
> The database must be stopped before running an import. The `--overwrite-destination` flag will erase any existing data in the target database.
 
```bash
sudo JDK_JAVA_OPTIONS="--add-opens=java.base/java.nio=ALL-UNNAMED --add-opens=java.base/java.lang=ALL-UNNAMED" \
  neo4j-admin database import full neo4j \
    --verbose \
    --multiline-fields=true \
    --overwrite-destination \
    --skip-bad-relationships \
    --nodes=PubMed:PubMedArticle=./PubMedArticle.csv \
    --nodes=PubMed:PubMedAgent:PubMedCollective=./PubMedCollective.csv \
    --nodes=PubMed:PubMedAgent:PubMedPerson=./PubMedPerson.csv \
    --nodes=PubMed:PubMedJournal=./PubMedJournal.csv \
    --nodes=PubMed:PubMedKeyword=./PubMedKeyword.csv \
    --nodes=MeSH:MeSHDescriptor=./MeSHDescriptor.csv \
    --nodes=MeSH:MeSHQQualifier=./MeSHQualifier.csv \
    --nodes=MeSH:MeSHSupplemental=./MeSHSupplemental.csv \
    --nodes=MeSH:MeSHConcept=./MeSHConcept.csv \
    --nodes=MeSH:MeSHDescriptorQualified=./MeSHDescriptorQualified.csv \
    --relationships=HAS_AUTHOR=./HAS_AUTHOR.csv \
    --relationships=IS_PART_OF=./IS_PART_OF.csv \
    --relationships=HAS_KEYWORD=./HAS_KEYWORD.csv \
    --relationships=CITES=./CITES.csv \
    --relationships=NARROWER_THAN=./NARROWER_THAN.csv \
    --relationships=BROADER_THAN=./BROADER_THAN.csv \
    --relationships=RELATED_TO=./RELATED_TO.csv \
    --relationships=HAS_MESH=./HAS_MESH.csv \
    --relationships=HAS_SUPPLEMENTARY_MESH=./HAS_SUPPLEMENTARY_MESH.csv \
    --relationships=HAS_DESCRIPTOR=./HAS_DESCRIPTOR.csv \
    --relationships=HAS_QUALIFIER=./HAS_QUALIFIER.csv \
    --relationships=MAPPED_TO=./MAPPED_TO.csv \
    --relationships=HAS_PHARMACOLOGICAL_ACTION=./HAS_PHARMACOLOGICAL_ACTION.csv \
    --relationships=HAS_CONCEPT=./HAS_CONCEPT.csv \
    --additional-config=/var/lib/neo4j/conf/neo4j.conf
```
 
The two `--add-opens` JVM flags are required on recent JDK versions to allow Neo4j's importer to access internal NIO and language APIs. Adjust `--additional-config` to point to your actual `neo4j.conf` if it lives elsewhere.

## Library Crates

Both library crates can be used independently in other projects.

### `mesh`

A streaming client for the NLM MeSH XML files. It fetches each file over HTTP, parses it with `quick-xml` and `serde`, and forwards each record to a caller-supplied callback without ever holding the full dataset in memory.

See [`crates/mesh/README.md`](crates/mesh/README.md) for the full API documentation.

### `pubmed`

An asynchronous client for the PubMed baseline. It discovers all available `.xml.gz` chunks, downloads them (with optional caching), decompresses, and parses them. Parallelism is left to the caller via `futures::stream`.

See [`crates/pubmed/README.md`](crates/pubmed/README.md) for the full API documentation.

## Feature Flags

Both `mesh` and `pubmed` expose a `debug_path` feature. When enabled, XML parse errors include the exact element path where the failure occurred (e.g. `PubmedArticleSet -> PubmedArticle[42] -> MedlineCitation`). This is useful during development but adds overhead; leave it disabled in production.

```toml
pm2kg = { ..., features = ["debug_path"] }
```

## License

This project is licensed under the [MIT License](LICENSE).
