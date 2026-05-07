//! Module of a SaverPubMed to CSV

use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use fxhash::FxHashSet;
use pubmed::chunks::models::{
    ArticleIdType, Author, AuthorType, DateYMD, Identifier, Journal, Keyword,
    KeywordListOwner, MedlineJournalInfo, MeshHeading, PubmedArticle,
    QualifierName, SupplMeshName, YN,
};

use crate::{saver::Writer, writer};

/// Struct that regroup CSV Files
#[derive(Debug)]
pub struct SaverPubMed {
    /// CSV Writer for PubMedArticle Nodes
    articles: Writer,

    /// CSV Writer for PubMedPerson Nodes
    persons: Writer,
    /// Set of the ID of saved person node
    person_id: Mutex<FxHashSet<String>>,

    /// CSV Writer for PubMedCollective Nodes
    collectives: Writer,
    /// Set of the ID of saved collective node
    collective_id: Mutex<FxHashSet<String>>,

    /// CSV Writer for PubMedJournal Nodes
    journals: Writer,
    /// Set of the ID of saved journal node
    journal_id: Mutex<FxHashSet<String>>,

    /// CSV Writer for PubMedKeyword Nodes
    keywords: Writer,
    /// Set of the ID of saved keyword node
    keyword_id: Mutex<FxHashSet<String>>,

    /// CSV Writer for MeSHDescriptorQualified Nodes
    pub(crate) mesh_qualifieds: Writer,
    /// Set of the ID of saved MeSHDescriptorQualified node
    pub(crate) qualified_id: Mutex<FxHashSet<String>>,

    /// CSV Writer for HAS_AUTHOR Relation
    has_author: Writer,

    /// CSV writer for IS_PART_OF Relation
    is_part_of: Writer,

    /// CSV writer for HAS_KEYWORD Relation
    has_keyword: Writer,

    /// CSV writer for CITES Relation
    cites: Writer,

    /// CSV Writer for HAS_MESH Relation
    has_mesh: Writer,

    /// CSV Writer for HAS_SUPPLEMENTARY_MESH Relation
    has_supplementary_mesh: Writer,

    /// CSV Writer for HAS_DESCRIPTOR Relation
    pub(crate) has_descriptor: Writer,

    /// CSV Writer for HAS_QUALIFIER Relation
    pub(crate) has_qualifier: Writer,
}

impl SaverPubMed {
    /// Init a saver which creates CSV file and writes header
    pub fn new(dir: &Path) -> std::io::Result<Self> {
        Ok(Self {
            articles: writer!(
                dir,
                "PubMedArticle",
                [
                    "pmid:ID(PubMedArticle){id-type: long}",
                    "title",
                    "abstract",
                    "dateCompleted:DATE",
                    "dateRevised:DATE",
                ]
            ),
            persons: writer!(
                dir,
                "PubMedPerson",
                [
                    ":ID(PubMedAgent)",
                    "lastName",
                    "foreName",
                    "initials",
                    "suffix",
                    "orcid",
                ]
            ),
            person_id: Mutex::new(FxHashSet::with_capacity_and_hasher(
                35_000_000,
                Default::default(),
            )),
            collectives: writer!(
                dir,
                "PubMedCollective",
                ["name:ID(PubMedAgent)",]
            ),
            collective_id: Mutex::new(FxHashSet::with_capacity_and_hasher(
                241_000,
                Default::default(),
            )),
            journals: writer!(
                dir,
                "PubMedJournal",
                [
                    ":ID(PubMedJournal)",
                    "title",
                    "country",
                    "nlmId",
                    "issn",
                    "isoAbbreviation",
                ]
            ),
            journal_id: Mutex::new(FxHashSet::with_capacity_and_hasher(
                50_000,
                Default::default(),
            )),
            keywords: writer!(
                dir,
                "PubMedKeyword",
                [":ID(PubMedKeyword)", "value", "supplier",]
            ),
            keyword_id: Mutex::new(FxHashSet::with_capacity_and_hasher(
                11_000_000,
                Default::default(),
            )),
            mesh_qualifieds: writer!(
                dir,
                "MeSHDescriptorQualified",
                [":ID(MeSHDescriptorQualified)"]
            ),
            qualified_id: Mutex::new(FxHashSet::with_capacity_and_hasher(
                6_000_000,
                Default::default(),
            )),
            has_author: writer!(
                dir,
                "HAS_AUTHOR",
                [":START_ID(PubMedArticle)", ":END_ID(PubMedAgent)",]
            ),
            is_part_of: writer!(
                dir,
                "IS_PART_OF",
                [":START_ID(PubMedArticle)", ":END_ID(PubMedJournal)",]
            ),
            has_keyword: writer!(
                dir,
                "HAS_KEYWORD",
                [":START_ID(PubMedArticle)", ":END_ID(PubMedKeyword)",]
            ),
            cites: writer!(
                dir,
                "CITES",
                [":START_ID(PubMedArticle)", ":END_ID(PubMedArticle)",]
            ),
            has_mesh: writer!(
                dir,
                "HAS_MESH",
                [
                    ":START_ID(PubMedArticle)",
                    "descriptorIsMajorTopic:boolean",
                    "qualifierMajorTopics:string[]",
                    ":END_ID(MeSHDescriptorQualified)",
                ]
            ),
            has_supplementary_mesh: writer!(
                dir,
                "HAS_SUPPLEMENTARY_MESH",
                [":START_ID(PubMedArticle)", ":END_ID(MeSH)",]
            ),
            has_descriptor: writer!(
                dir,
                "HAS_DESCRIPTOR",
                [":START_ID(MeSHDescriptorQualified)", ":END_ID(MeSH)",]
            ),
            has_qualifier: writer!(
                dir,
                "HAS_QUALIFIER",
                [":START_ID(MeSHDescriptorQualified)", ":END_ID(MeSH)",]
            ),
        })
    }

    /// Flush every CSV file
    pub fn flush(&self) -> std::io::Result<()> {
        self.articles.lock().unwrap().flush()?;
        self.persons.lock().unwrap().flush()?;
        self.collectives.lock().unwrap().flush()?;
        self.journals.lock().unwrap().flush()?;
        self.keywords.lock().unwrap().flush()?;
        self.mesh_qualifieds.lock().unwrap().flush()?;
        self.has_author.lock().unwrap().flush()?;
        self.is_part_of.lock().unwrap().flush()?;
        self.has_keyword.lock().unwrap().flush()?;
        self.cites.lock().unwrap().flush()?;
        self.has_mesh.lock().unwrap().flush()?;
        self.has_supplementary_mesh.lock().unwrap().flush()?;
        self.has_descriptor.lock().unwrap().flush()?;
        self.has_qualifier.lock().unwrap().flush()?;

        Ok(())
    }

    /// Save one MeSH Supplementary
    pub fn add_supplementary_mesh(
        &self,
        mesh: &SupplMeshName,
        pmid: &str,
    ) -> std::io::Result<()> {
        self.has_supplementary_mesh
            .lock()
            .unwrap()
            .write_record([pmid, mesh.ui.as_str()])?;
        Ok(())
    }

    /// Save one MeSH
    pub fn add_mesh(
        &self,
        mesh: &MeshHeading,
        pmid: &str,
    ) -> std::io::Result<()> {
        let mut quals: Vec<&str> = mesh
            .qualifier_names
            .iter()
            .map(|q: &QualifierName| q.ui.as_str())
            .collect();
        quals.sort();
        let quals: String = quals.join(":");
        let mesh_id: String = format!("{}-{}", mesh.descriptor_name.ui, quals);

        self.has_mesh.lock().unwrap().write_record([
            pmid,
            if matches!(mesh.descriptor_name.major_topic_yn, YN::Y) {
                "true"
            } else {
                "false"
            },
            mesh.qualifier_names
                .iter()
                .filter_map(|qual| {
                    matches!(qual.major_topic_yn, YN::Y)
                        .then_some(qual.ui.as_str())
                })
                .collect::<Vec<&str>>()
                .join(";")
                .as_str(),
            &mesh_id,
        ])?;

        let mut qualified_id: MutexGuard<_> = self.qualified_id.lock().unwrap();

        if !qualified_id.contains(&mesh_id) {
            self.mesh_qualifieds
                .lock()
                .unwrap()
                .write_record([&mesh_id])?;

            self.has_descriptor
                .lock()
                .unwrap()
                .write_record([&mesh_id, mesh.descriptor_name.ui.as_str()])?;

            for qual in mesh.qualifier_names.iter() {
                self.has_qualifier
                    .lock()
                    .unwrap()
                    .write_record([&mesh_id, qual.ui.as_str()])?;
            }

            qualified_id.insert(mesh_id);
        }

        Ok(())
    }

    /// Save one citation
    pub fn add_cites(&self, source: &str, to: &str) -> std::io::Result<()> {
        self.cites.lock().unwrap().write_record([source, to])?;
        Ok(())
    }

    /// Save one keyword
    pub fn add_keyword(
        &self,
        keyword: &Keyword,
        owner: &KeywordListOwner,
        pmid: &str,
    ) -> std::io::Result<()> {
        if keyword.content.is_empty() {
            return Ok(())
        }

        let keyword_id: String =
            format!("{}-{}", owner.as_str(), keyword.content);

        self.has_keyword
            .lock()
            .unwrap()
            .write_record([pmid, &keyword_id])?;

        let mut kid: MutexGuard<_> = self.keyword_id.lock().unwrap();

        if !kid.contains(&keyword_id) {
            self.keywords.lock().unwrap().write_record([
                &keyword_id,
                keyword.content.as_str(),
                owner.as_str(),
            ])?;

            kid.insert(keyword_id);
        }
        Ok(())
    }

    /// Save one author
    pub fn add_author(
        &self,
        author: &Author,
        pmid: &str,
    ) -> std::io::Result<()> {
        match &author.name {
            AuthorType::Person {
                last_name,
                fore_name,
                initials,
                suffix,
            } => {
                let fore_name: &str = fore_name.as_deref().unwrap_or("");
                let suffix: &str = suffix.as_deref().unwrap_or("");
                let orcid: Option<&str> =
                    author.identifiers.iter().find_map(|id| match id {
                        Identifier::Orcid(id) if !id.is_empty() => {
                            Some(id.as_str())
                        }
                        _ => None,
                    });
                let person_id: String = match orcid {
                    Some(id) => format!("ORCID:{id}"),
                    None => format!("{}-{}-{}", fore_name, last_name, suffix),
                };

                self.has_author
                    .lock()
                    .unwrap()
                    .write_record([pmid, &person_id])?;

                let mut pid: MutexGuard<_> = self.person_id.lock().unwrap();

                if !pid.contains(&person_id) {
                    self.persons.lock().unwrap().write_record([
                        &person_id,
                        last_name,
                        fore_name,
                        initials.as_deref().unwrap_or(""),
                        suffix,
                        orcid.unwrap_or(""),
                    ])?;

                    pid.insert(person_id);
                }
            }
            AuthorType::Collective { name } => {
                self.has_author.lock().unwrap().write_record([pmid, name])?;

                let mut collective_id: MutexGuard<_> =
                    self.collective_id.lock().unwrap();

                if !collective_id.contains(name) {
                    collective_id.insert(name.clone());
                    self.collectives.lock().unwrap().write_record([name])?;
                }
            }
        }

        Ok(())
    }

    /// Save one journal
    pub fn add_journal(
        &self,
        journal: &Journal,
        journal_info: &MedlineJournalInfo,
        pmid: &str,
    ) -> std::io::Result<()> {
        let journal_id: &str = match &journal_info.nlm_unique_id {
            Some(nlm) => nlm,
            None => &journal_info.medline_ta,
        };

        self.is_part_of
            .lock()
            .unwrap()
            .write_record([pmid, journal_id])?;

        let mut jid: MutexGuard<_> = self.journal_id.lock().unwrap();

        if !jid.contains(journal_id) {
            self.journals.lock().unwrap().write_record([
                journal_id,
                journal.title.as_deref().unwrap_or(""),
                journal_info.country.as_deref().unwrap_or(""),
                journal_info.nlm_unique_id.as_deref().unwrap_or(""),
                journal
                    .issn
                    .as_ref()
                    .map(|i| i.value.as_str())
                    .unwrap_or(""),
                journal.iso_abbreviation.as_deref().unwrap_or(""),
            ])?;

            jid.insert(journal_id.to_string());
        }

        Ok(())
    }

    /// Save one article
    pub fn add_article(&self, article: &PubmedArticle) -> std::io::Result<()> {
        let pmid: String = article.medline_citation.pmid.value.to_string();

        let abstract_text: String = article
            .medline_citation
            .article
            .abstract_
            .as_ref()
            .map(|a| {
                a.texts
                    .iter()
                    .map(|t| t.content.as_str())
                    .collect::<Vec<&str>>()
                    .join("\n\n")
            })
            .unwrap_or_default();

        self.articles.lock().unwrap().write_record([
            &pmid,
            &article.medline_citation.article.title.content,
            &abstract_text,
            &date_to_string(article.medline_citation.date_completed.as_ref()),
            &date_to_string(article.medline_citation.date_revised.as_ref()),
        ])?;

        if let Some(authors) = &article.medline_citation.article.author_list {
            for author in authors.authors.iter() {
                self.add_author(author, &pmid)?;
            }
        }

        for keywords in article.medline_citation.keyword_lists.iter() {
            for keyword in keywords.keywords.iter() {
                self.add_keyword(keyword, &keywords.owner, &pmid)?;
            }
        }

        if let Some(meshs) = &article.medline_citation.mesh_heading_list {
            for mesh in meshs.headings.iter() {
                self.add_mesh(mesh, &pmid)?;
            }
        }

        if let Some(suppls) = &article.medline_citation.suppl_mesh_list {
            for mesh in suppls.names.iter() {
                self.add_supplementary_mesh(mesh, &pmid)?;
            }
        }

        if let Some(pubmed) = &article.pubmed_data {
            for cites_list in pubmed.reference_lists.iter() {
                for cites in cites_list.references.iter() {
                    if let Some(cite) =
                        cites.article_id_list.as_ref().and_then(|l| {
                            l.ids.iter().find(|id| {
                                matches!(id.id_type, ArticleIdType::PUBMED)
                                    && id.value.parse::<u32>().is_ok()
                            })
                        })
                    {
                        self.add_cites(&pmid, &cite.value)?;
                    }
                }
            }
        }

        self.add_journal(
            &article.medline_citation.article.journal,
            &article.medline_citation.medline_journal_info,
            &pmid,
        )?;

        Ok(())
    }
}

/// Converts an optional date reference into a formatted ISO 8601 string.
///
/// This function takes an `Option<&DateYMD>` and returns a string in the
/// format `YYYY-MM-DD`. If the input is `None`, it returns an empty string.
fn date_to_string(d: Option<&DateYMD>) -> String {
    d.map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
        .unwrap_or_default()
}
