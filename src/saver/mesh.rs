//! Module of a SaverPubMeSH to CSV

use std::{
    borrow::Cow,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use fxhash::{FxHashMap, FxHashSet};
use mesh::{
    descriptor::models::{
        ConceptList, DescriptorRecord, PharmacologicalActionList, RelationName,
        YesNo,
    },
    qualifier::models::QualifierRecord,
    supplemental::models::SupplementalRecord,
};

use crate::{saver::Writer, writer};

/// Struct that regroup CSV Files
pub struct SaverMeSH<'a> {
    /// CSV Writer for MeSHDescriptor Nodes
    mesh_descriptors: Writer,

    /// CSV Writer for MeSHQualifier Nodes
    mesh_qualifiers: Writer,

    /// CSV Writer for MeSHSupplemental Nodes
    mesh_supplemental: Writer,

    /// CSV Writer for MeSHConcept Nodes
    mesh_concept: Writer,

    /// CSV Writer for MeSHQualified Nodes
    mesh_qualifieds: Writer,
    /// Set of the ID of saved MeSHQualified node
    qualified_id: Mutex<FxHashSet<String>>,

    /// CSV writer for NARROWER_THAN Relation for Descriptor Node
    narrower_than: Writer,

    /// MeSH path to his DUI
    path_to_id: Mutex<FxHashMap<Arc<str>, Arc<str>>>,

    /// MeSH DUI to his path
    #[allow(clippy::type_complexity)]
    id_to_path: Mutex<FxHashMap<Arc<str>, Vec<Arc<str>>>>,

    /// CSV Writer for BROADER_THAN Relation
    broader_than: Writer,

    /// CSV Writer for RELATED_TO Relation
    related_to: Writer,

    /// Set of relation Broader Narrower Related saved
    #[allow(clippy::type_complexity)]
    nbr: Mutex<FxHashSet<(Cow<'a, str>, RelationName, Cow<'a, str>)>>,

    /// CSV Writer for HAS_DESCRIPTOR Relation
    has_descriptor: Writer,

    /// CSV Writer for HAS_QUALIFIER Relation
    has_qualifier: Writer,

    /// CSV Writer for HAS_PHARMACOLOGICAL_ACTION Relation
    has_pa: Writer,

    /// CSV Writer for HAS_CONCEPT Relation
    has_concept: Writer,

    /// CSV writer for MAPPED_TO Relation
    mapped_to: Writer,
}

impl<'a> SaverMeSH<'a> {
    /// Init a saver which creates CSV file and writes header
    pub fn new(
        dir: &Path,
        mesh_qualifieds: Writer,
        qualified_id: Mutex<FxHashSet<String>>,
        has_descriptor: Writer,
        has_qualifier: Writer,
    ) -> std::io::Result<Self> {
        Ok(Self {
            mesh_descriptors: writer!(
                dir,
                "MeSHDescriptor",
                ["ui:ID(MeSH)", "name", "class:int", "treePath:string[]"]
            ),
            mesh_qualifiers: writer!(
                dir,
                "MeSHQualifier",
                ["ui:ID(MeSH)", "name", "treePath:string[]"]
            ),
            mesh_supplemental: writer!(
                dir,
                "MeSHSupplemental",
                ["ui:ID(MeSH)", "class:int", "name"]
            ),
            mesh_concept: writer!(
                dir,
                "MeSHConcept",
                ["ui:ID(MeSH)", "CASNo1Name", "name"]
            ),
            mesh_qualifieds,
            qualified_id,
            narrower_than: writer!(
                dir,
                "NARROWER_THAN",
                [":START_ID(MeSH)", ":END_ID(MeSH)"]
            ),
            path_to_id: Default::default(),
            id_to_path: Default::default(),
            broader_than: writer!(
                dir,
                "BROADER_THAN",
                [":START_ID(MeSH)", ":END_ID(MeSH)"]
            ),
            related_to: writer!(
                dir,
                "RELATED_TO",
                [":START_ID(MeSH)", ":END_ID(MeSH)"]
            ),
            nbr: Default::default(),
            has_descriptor,
            has_qualifier,
            has_pa: writer!(
                dir,
                "HAS_PHARMACOLOGICAL_ACTION",
                [":START_ID(MeSH)", ":END_ID(MeSH)",]
            ),
            has_concept: writer!(
                dir,
                "HAS_CONCEPT",
                [":START_ID(MeSH)", "isPreferred:boolean", ":END_ID(MeSH)",]
            ),
            mapped_to: writer!(
                dir,
                "MAPPED_TO",
                [
                    ":START_ID(MeSH)",
                    "descriptorIsMajorTopic:boolean",
                    ":END_ID(MeSHQualified)"
                ]
            ),
        })
    }

    /// Flush every CSV file
    pub fn flush(&self) -> std::io::Result<()> {
        self.mesh_descriptors.lock().unwrap().flush()?;
        self.mesh_qualifiers.lock().unwrap().flush()?;
        self.mesh_supplemental.lock().unwrap().flush()?;
        self.mesh_qualifieds.lock().unwrap().flush()?;
        self.broader_than.lock().unwrap().flush()?;
        self.related_to.lock().unwrap().flush()?;
        self.has_descriptor.lock().unwrap().flush()?;
        self.has_qualifier.lock().unwrap().flush()?;
        self.has_pa.lock().unwrap().flush()?;
        self.has_concept.lock().unwrap().flush()?;
        self.mapped_to.lock().unwrap().flush()?;

        let path_to_id: MutexGuard<_> = self.path_to_id.lock().unwrap();
        let id_to_path: MutexGuard<_> = self.id_to_path.lock().unwrap();

        for (id, paths) in id_to_path.iter() {
            let mut parents: FxHashSet<&str> = FxHashSet::default();
            for path in paths {
                if let Some(parent) = remove_last_extension(path) {
                    let parent_id: &str =
                        path_to_id.get(parent).unwrap().as_ref();
                    if !parents.contains(parent_id) {
                        self.narrower_than
                            .lock()
                            .unwrap()
                            .write_record([parent_id, id.as_ref()])?;

                        parents.insert(parent_id);
                    }
                }
            }
        }

        self.narrower_than.lock().unwrap().flush()?;

        Ok(())
    }

    /// Save one list of concept action
    pub fn add_concepts(
        &self,
        id: &str,
        concepts: &ConceptList,
    ) -> std::io::Result<()> {
        for concept in concepts.items.iter() {
            self.mesh_concept.lock().unwrap().write_record([
                concept.ui.as_str(),
                concept.casn1_name.as_deref().unwrap_or(""),
                concept.name.value.as_str(),
            ])?;

            self.has_concept.lock().unwrap().write_record([
                id,
                if matches!(concept.preferred_concept_yn, YesNo::Y) {
                    "true"
                } else {
                    "false"
                },
                concept.ui.as_str(),
            ])?;

            if let Some(rels) = &concept.concept_relations {
                for rel in rels.items.iter() {
                    let name: &RelationName = match &rel.name {
                        Some(
                            n @ (RelationName::NRW
                            | RelationName::BRD
                            | RelationName::REL),
                        ) => n,
                        _ => continue,
                    };

                    let c1: &str = rel.concept1.as_str();
                    let c2: &str = rel.concept2.as_str();

                    let mut nbr: MutexGuard<_> = self.nbr.lock().unwrap();

                    if !nbr.contains(&(
                        Cow::Borrowed(c2),
                        name.clone(),
                        Cow::Borrowed(c1),
                    )) {
                        let writer = match name {
                            RelationName::NRW => &self.narrower_than,
                            RelationName::BRD => &self.broader_than,
                            RelationName::REL => &self.related_to,
                        };

                        writer.lock().unwrap().write_record([c2, c1])?;
                        nbr.insert((
                            Cow::Owned(c2.to_string()),
                            name.clone(),
                            Cow::Owned(c1.to_string()),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Save one list of pharmacologial action
    pub fn add_pa(
        &self,
        id: &str,
        pal: &Option<PharmacologicalActionList>,
    ) -> std::io::Result<()> {
        if let Some(pal) = pal {
            for pa in pal.items.iter() {
                self.has_pa.lock().unwrap().write_record([
                    id,
                    pa.descriptor_referred_to.ui.as_str(),
                ])?;
            }
        }

        Ok(())
    }

    /// Save one supplemental
    pub fn add_supplemental(
        &self,
        supp: &SupplementalRecord,
    ) -> std::io::Result<()> {
        self.mesh_supplemental.lock().unwrap().write_record([
            supp.ui.as_str(),
            supp.class.as_deref().unwrap_or(""),
            supp.name.value.as_str(),
        ])?;

        self.add_concepts(supp.ui.as_str(), &supp.concepts)?;
        self.add_pa(supp.ui.as_str(), &supp.pharmacological_actions)?;

        if let Some(mapped) = &supp.heading_maps {
            for map in mapped.items.iter() {
                let mesh_id: String = format!(
                    "{}-{}",
                    map.descriptor.ui,
                    map.qualifier
                        .as_ref()
                        .map(|q| q.ui.as_str())
                        .unwrap_or_default()
                );

                self.mapped_to.lock().unwrap().write_record([
                    supp.ui.as_str(),
                    map.descriptor.ui.starts_with('*').to_string().as_str(),
                    &mesh_id,
                ])?;

                let mut qualified_id: MutexGuard<_> =
                    self.qualified_id.lock().unwrap();

                if !qualified_id.contains(&mesh_id) {
                    self.mesh_qualifieds
                        .lock()
                        .unwrap()
                        .write_record([&mesh_id])?;

                    self.has_descriptor.lock().unwrap().write_record([
                        &mesh_id,
                        map.descriptor.ui.trim_start_matches('*'),
                    ])?;

                    if let Some(qual) = &map.qualifier {
                        self.has_qualifier
                            .lock()
                            .unwrap()
                            .write_record([&mesh_id, qual.ui.as_str()])?;
                    }

                    qualified_id.insert(mesh_id);
                }
            }
        }

        Ok(())
    }

    /// Save one descriptor
    pub fn add_descriptor(
        &self,
        desc: &DescriptorRecord,
    ) -> std::io::Result<()> {
        self.mesh_descriptors.lock().unwrap().write_record([
            desc.ui.as_str(),
            desc.name.value.as_str(),
            desc.class.as_deref().unwrap_or(""),
            desc.tree_numbers
                .as_ref()
                .map(|t| {
                    let ui: Arc<str> = desc.ui.clone().into();
                    let paths: Vec<Arc<str>> = t
                        .items
                        .iter()
                        .map(|item| {
                            let item: Arc<str> = item.clone().into();

                            self.path_to_id
                                .lock()
                                .unwrap()
                                .insert(Arc::clone(&item), Arc::clone(&ui));

                            item
                        })
                        .collect();

                    self.id_to_path.lock().unwrap().insert(ui, paths);

                    t.items.join(";")
                })
                .unwrap_or_default()
                .as_str(),
        ])?;

        self.add_concepts(desc.ui.as_str(), &desc.concepts)?;
        self.add_pa(desc.ui.as_str(), &desc.pharmacological_actions)?;

        Ok(())
    }

    /// Save one qualifier
    pub fn add_qualifier(&self, qual: &QualifierRecord) -> std::io::Result<()> {
        self.mesh_qualifiers.lock().unwrap().write_record([
            qual.ui.as_str(),
            qual.name.value.as_str(),
            qual.tree_numbers
                .as_ref()
                .map(|t| {
                    let ui: Arc<str> = qual.ui.clone().into();
                    let paths: Vec<Arc<str>> = t
                        .items
                        .iter()
                        .map(|item| {
                            let item: Arc<str> = item.clone().into();

                            self.path_to_id
                                .lock()
                                .unwrap()
                                .insert(Arc::clone(&item), Arc::clone(&ui));

                            item
                        })
                        .collect();

                    self.id_to_path.lock().unwrap().insert(ui, paths);

                    t.items.join(";")
                })
                .unwrap_or_default()
                .as_str(),
        ])?;

        self.add_concepts(qual.ui.as_str(), &qual.concepts)?;

        Ok(())
    }
}

/// Remove the last .<something> of `s` and None if `s` don't contains this.
fn remove_last_extension(s: &str) -> Option<&str> {
    let dot_pos: usize = s.rfind('.')?;
    Some(&s[..dot_pos])
}
