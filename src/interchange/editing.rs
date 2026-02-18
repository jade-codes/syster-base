//! Change tracking and mutation API for [`Model`].
//!
//! `ChangeTracker` records which elements and relationships were
//! created, modified, or removed. It integrates with [`ModelHost`]
//! via the `session()` method, enabling edit→render workflows.
//!
//! ## Example
//!
//! ```ignore
//! use syster::ide::AnalysisHost;
//!
//! let mut host = AnalysisHost::new();
//! host.set_file_content("model.sysml", "package P { part def A; }");
//!
//! let result = host.apply_model_edit("model.sysml", |model, tracker| {
//!     let a_id = model.find_by_name("A")[0].id().clone();
//!     tracker.rename(model, &a_id, "B");
//! });
//!
//! // See what changed
//! assert!(tracker.is_dirty(&a_id));
//! let dirty = tracker.dirty_elements();
//! assert_eq!(dirty.len(), 1);
//! ```

use super::model::{Element, ElementId, ElementKind, Model, PropertyValue};
use std::collections::HashSet;
use std::sync::Arc;

/// Tracks mutations applied to a [`Model`].
///
/// Create a tracker, apply mutations through its methods (which
/// delegate to the model), then query which elements are dirty.
/// This is the input to the region re-renderer (Phase D).
#[derive(Clone, Debug, Default)]
pub struct ChangeTracker {
    /// Element IDs that have been modified.
    modified: HashSet<ElementId>,
    /// Element IDs that have been created (subset of modified).
    created: HashSet<ElementId>,
    /// Element IDs that have been removed.
    removed: HashSet<ElementId>,
    /// Relationship element IDs that were added.
    added_relationships: HashSet<ElementId>,
}

impl ChangeTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all dirty state.
    pub fn clear(&mut self) {
        self.modified.clear();
        self.created.clear();
        self.removed.clear();
        self.added_relationships.clear();
    }

    // ── Query ───────────────────────────────────────────────────────

    /// Whether any mutations have been recorded.
    pub fn has_changes(&self) -> bool {
        !self.modified.is_empty() || !self.removed.is_empty()
    }

    /// Whether a specific element was modified (including creation).
    pub fn is_dirty(&self, id: &ElementId) -> bool {
        self.modified.contains(id)
    }

    /// Whether a specific element was newly created.
    pub fn is_created(&self, id: &ElementId) -> bool {
        self.created.contains(id)
    }

    /// Whether a specific element was removed.
    pub fn is_removed(&self, id: &ElementId) -> bool {
        self.removed.contains(id)
    }

    /// All dirty (modified or created) element IDs.
    pub fn dirty_elements(&self) -> Vec<&ElementId> {
        self.modified.iter().collect()
    }

    /// All created element IDs.
    pub fn created_elements(&self) -> Vec<&ElementId> {
        self.created.iter().collect()
    }

    /// All removed element IDs.
    pub fn removed_elements(&self) -> Vec<&ElementId> {
        self.removed.iter().collect()
    }

    // ── Mutations ───────────────────────────────────────────────────

    /// Mark an element as dirty (for external mutations).
    pub fn mark_dirty(&mut self, id: &ElementId) {
        self.modified.insert(id.clone());
    }

    /// Rename an element.
    pub fn rename(&mut self, model: &mut Model, id: &ElementId, new_name: &str) {
        if let Some(el) = model.get_mut(id) {
            el.name = Some(Arc::from(new_name));
            // Update qualified name if it had one
            if let Some(qn) = &el.qualified_name {
                if let Some(pos) = qn.rfind("::") {
                    let prefix = &qn[..pos];
                    el.qualified_name = Some(Arc::from(format!("{prefix}::{new_name}")));
                } else {
                    el.qualified_name = Some(Arc::from(new_name));
                }
            }
            self.modified.insert(id.clone());
        }
    }

    /// Set an element's short name.
    pub fn set_short_name(&mut self, model: &mut Model, id: &ElementId, short_name: Option<&str>) {
        if let Some(el) = model.get_mut(id) {
            el.short_name = short_name.map(Arc::from);
            self.modified.insert(id.clone());
        }
    }

    /// Set a boolean property on an element.
    pub fn set_abstract(&mut self, model: &mut Model, id: &ElementId, value: bool) {
        if let Some(el) = model.get_mut(id) {
            el.is_abstract = value;
            el.properties
                .insert(Arc::from("isAbstract"), PropertyValue::Boolean(value));
            self.modified.insert(id.clone());
        }
    }

    /// Set the `is_variation` flag.
    pub fn set_variation(&mut self, model: &mut Model, id: &ElementId, value: bool) {
        if let Some(el) = model.get_mut(id) {
            el.is_variation = value;
            el.properties
                .insert(Arc::from("isVariation"), PropertyValue::Boolean(value));
            self.modified.insert(id.clone());
        }
    }

    /// Set a property value.
    pub fn set_property(
        &mut self,
        model: &mut Model,
        id: &ElementId,
        key: &str,
        value: PropertyValue,
    ) {
        if let Some(el) = model.get_mut(id) {
            el.properties.insert(Arc::from(key), value);
            self.modified.insert(id.clone());
        }
    }

    /// Set the documentation text.
    pub fn set_documentation(&mut self, model: &mut Model, id: &ElementId, doc: Option<&str>) {
        if let Some(el) = model.get_mut(id) {
            el.documentation = doc.map(Arc::from);
            self.modified.insert(id.clone());
        }
    }

    /// Add a new element to the model.
    /// If `owner_id` is Some, the element will be owned by that element.
    ///
    /// Non-relationship elements are automatically wrapped in an
    /// `OwningMembership` or `FeatureMembership` intermediary (per KerML),
    /// unless the parent is already a membership.
    pub fn add_element(
        &mut self,
        model: &mut Model,
        mut element: Element,
        owner_id: Option<&ElementId>,
    ) -> ElementId {
        let id = element.id.clone();

        if let Some(parent_id) = owner_id {
            // Decide whether to wrap in a membership.
            // Skip wrapping when:
            //  - the child is itself a relationship or membership
            //  - the parent is already a membership (already wrapped)
            //  - the parent is a relationship (relationships own their
            //    ownedRelatedElement directly, e.g. FeatureValue → LiteralString)
            let needs_wrap = !element.kind.is_relationship()
                && !element.kind.is_membership()
                && model.get(parent_id).map_or(false, |p| {
                    !p.kind.is_membership() && !p.kind.is_relationship()
                });

            if needs_wrap {
                // Create a membership wrapper between parent and child
                let m_kind = ElementKind::membership_kind_for(element.kind);
                let m_id = ElementId::new(format!("{}-m", id.as_str()));

                let mut membership = Element::new(m_id.clone(), m_kind);
                membership.owner = Some(parent_id.clone());
                membership.owned_elements.push(id.clone());

                // Child is owned by the membership, not the parent
                element.owner = Some(m_id.clone());

                // Parent owns the membership
                if let Some(parent) = model.get_mut(parent_id) {
                    parent.owned_elements.push(m_id.clone());
                    self.modified.insert(parent_id.clone());
                }

                model.add_element(membership);
            } else {
                // Relationship or parent is already a membership — direct child
                element.owner = Some(parent_id.clone());
                if let Some(parent) = model.get_mut(parent_id) {
                    parent.owned_elements.push(id.clone());
                    self.modified.insert(parent_id.clone());
                }
            }
        }

        model.add_element(element);
        self.created.insert(id.clone());
        self.modified.insert(id.clone());
        id
    }

    /// Remove an element from the model.
    /// Also removes it from its owner's owned_elements list.
    /// If the owner is a membership wrapper that would be left empty,
    /// the membership is removed too.
    pub fn remove_element(&mut self, model: &mut Model, id: &ElementId) -> Option<Element> {
        // Check if the owner is a membership wrapper that would be left empty
        let membership_to_remove =
            model
                .get(id)
                .and_then(|el| el.owner.as_ref())
                .and_then(|owner_id| {
                    model
                        .get(owner_id)
                        .filter(|owner| {
                            owner.kind.is_membership() && owner.owned_elements.len() <= 1
                        })
                        .map(|_| owner_id.clone())
                });

        // Remove from parent's owned_elements
        if let Some(el) = model.get(id) {
            if let Some(owner_id) = el.owner.clone() {
                if let Some(parent) = model.get_mut(&owner_id) {
                    parent.owned_elements.retain(|child| child != id);
                    self.modified.insert(owner_id);
                }
            }
        }

        // If the owner was a membership that's now empty, remove it too
        if let Some(m_id) = membership_to_remove {
            if let Some(m_el) = model.get(&m_id) {
                if let Some(grandparent_id) = m_el.owner.clone() {
                    if let Some(gp) = model.get_mut(&grandparent_id) {
                        gp.owned_elements.retain(|child| *child != m_id);
                        self.modified.insert(grandparent_id);
                    }
                }
            }
            model.roots.retain(|r| *r != m_id);
            model.elements.swap_remove(&m_id);
            self.removed.insert(m_id);
        }

        // Remove from roots if it was a root
        model.roots.retain(|r| r != id);

        // Remove relationships involving this element
        // Remove relationship elements from the elements map
        let rel_element_ids: Vec<ElementId> = model
            .elements
            .values()
            .filter(|e| {
                e.relationship
                    .as_ref()
                    .map_or(false, |rd| rd.source.contains(id) || rd.target.contains(id))
            })
            .map(|e| e.id.clone())
            .collect();
        for rel_id in rel_element_ids {
            model.elements.swap_remove(&rel_id);
        }

        let removed = model.elements.swap_remove(id);
        if removed.is_some() {
            self.removed.insert(id.clone());
            // Remove from modified/created if it was pending
            self.modified.remove(id);
            self.created.remove(id);
        }
        removed
    }

    /// Add a relationship between two elements.
    pub fn add_relationship(
        &mut self,
        model: &mut Model,
        id: impl Into<ElementId>,
        kind: ElementKind,
        source: impl Into<ElementId>,
        target: impl Into<ElementId>,
    ) {
        let source_id: ElementId = source.into();
        let rel_id = model.add_rel(id, kind, source_id.clone(), target, None);
        self.added_relationships.insert(rel_id);
        self.modified.insert(source_id);
    }

    /// Move an element to a new owner.
    pub fn reparent(&mut self, model: &mut Model, id: &ElementId, new_owner: &ElementId) {
        // Determine if the child is wrapped in a membership
        let membership_id = model
            .get(id)
            .and_then(|el| el.owner.as_ref())
            .and_then(|owner_id| {
                model
                    .get(owner_id)
                    .filter(|owner| owner.kind.is_membership())
                    .map(|_| owner_id.clone())
            });

        if let Some(m_id) = membership_id {
            // Move the membership wrapper (which carries the child)
            // Remove membership from its old parent (the logical grandparent)
            if let Some(m_el) = model.get(&m_id) {
                if let Some(gp_id) = m_el.owner.clone() {
                    if let Some(gp) = model.get_mut(&gp_id) {
                        gp.owned_elements.retain(|child| *child != m_id);
                        self.modified.insert(gp_id);
                    }
                }
            }
            // If the membership was a root, remove it from roots
            model.roots.retain(|r| *r != m_id);

            // Re-parent membership to new_owner
            if let Some(m_el) = model.get_mut(&m_id) {
                m_el.owner = Some(new_owner.clone());
            }
            if let Some(new_parent) = model.get_mut(new_owner) {
                new_parent.owned_elements.push(m_id.clone());
                self.modified.insert(new_owner.clone());
            }
        } else {
            // No membership wrapper — element is either a root or a relationship.
            // Detach from old parent (if any).
            if let Some(el) = model.get(id) {
                if let Some(old_owner_id) = el.owner.clone() {
                    if let Some(old_parent) = model.get_mut(&old_owner_id) {
                        old_parent.owned_elements.retain(|child| child != id);
                        self.modified.insert(old_owner_id);
                    }
                }
            }
            // If the element was a root, remove it from roots
            model.roots.retain(|r| r != id);

            // For non-relationship elements, create a membership wrapper
            // so the invariant holds (all content children are wrapped).
            let should_wrap = model.get(id).map_or(false, |el| {
                !el.kind.is_relationship() && !el.kind.is_membership()
            }) && model.get(new_owner).map_or(false, |p| {
                !p.kind.is_membership() && !p.kind.is_relationship()
            });

            if should_wrap {
                let kind = model.get(id).unwrap().kind;
                let m_kind = ElementKind::membership_kind_for(kind);
                let m_id = ElementId::new(format!("{}-m", id.as_str()));

                let mut membership = Element::new(m_id.clone(), m_kind);
                membership.owner = Some(new_owner.clone());
                membership.owned_elements.push(id.clone());

                if let Some(el) = model.get_mut(id) {
                    el.owner = Some(m_id.clone());
                }
                if let Some(new_parent) = model.get_mut(new_owner) {
                    new_parent.owned_elements.push(m_id.clone());
                    self.modified.insert(new_owner.clone());
                }
                model.add_element(membership);
            } else {
                // Relationship element — direct child, no wrapper needed
                if let Some(el) = model.get_mut(id) {
                    el.owner = Some(new_owner.clone());
                }
                if let Some(new_parent) = model.get_mut(new_owner) {
                    new_parent.owned_elements.push(id.clone());
                    self.modified.insert(new_owner.clone());
                }
            }
        }

        self.modified.insert(id.clone());
    }
}

// ── ModelHost integration ───────────────────────────────────────────

use super::host::ModelHost;

impl ModelHost {
    /// Create a change tracker for this host.
    /// Mutations go through the tracker which records what changed.
    pub fn tracker(&self) -> ChangeTracker {
        ChangeTracker::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::ElementKind;
    use super::*;

    fn host(source: &str) -> ModelHost {
        ModelHost::from_text(source).expect("should parse")
    }

    #[test]
    fn tracker_rename() {
        let mut h = host("package P { part def Vehicle; }");
        let mut t = h.tracker();

        let v_id = h.find_by_name("Vehicle")[0].id().clone();
        t.rename(h.model_mut(), &v_id, "Car");

        assert!(t.is_dirty(&v_id));
        assert_eq!(h.find_by_name("Car").len(), 1);
        assert_eq!(h.find_by_name("Vehicle").len(), 0);

        // Qualified name should be updated
        let found = h.find_by_name("Car")[0].qualified_name();
        assert!(
            found.map(|qn| qn.ends_with("Car")).unwrap_or(false),
            "qualified_name should end with Car, got {:?}",
            found,
        );
    }

    #[test]
    fn tracker_add_element() {
        let mut h = host("package P;");
        let mut t = h.tracker();

        let p_id = h.find_by_name("P")[0].id().clone();
        let new_el = Element::new("new1", ElementKind::PartDefinition).with_name("Widget");
        let new_id = t.add_element(h.model_mut(), new_el, Some(&p_id));

        assert!(t.is_created(&new_id));
        assert!(t.is_dirty(&new_id));
        assert!(t.is_dirty(&p_id), "parent should be dirty too");

        // Should be findable
        assert_eq!(h.find_by_name("Widget").len(), 1);

        // Should be owned
        let p = h.find_by_name("P")[0];
        let members = p.owned_members();
        assert!(members.iter().any(|m| m.name() == Some("Widget")));
    }

    #[test]
    fn tracker_remove_element() {
        let mut h = host("package P { part def A; part def B; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        let removed = t.remove_element(h.model_mut(), &a_id);
        assert!(removed.is_some());
        assert!(t.is_removed(&a_id));

        // Should no longer be findable
        assert_eq!(h.find_by_name("A").len(), 0);
        // B should still be there
        assert_eq!(h.find_by_name("B").len(), 1);
    }

    #[test]
    fn tracker_set_abstract() {
        let mut h = host("package P { part def Vehicle; }");
        let mut t = h.tracker();

        let v_id = h.find_by_name("Vehicle")[0].id().clone();
        assert!(!h.view(&v_id).unwrap().is_abstract());

        t.set_abstract(h.model_mut(), &v_id, true);
        assert!(t.is_dirty(&v_id));
        assert!(h.view(&v_id).unwrap().is_abstract());
    }

    #[test]
    fn tracker_reparent() {
        let mut h = host("package A { part def X; } package B;");
        let mut t = h.tracker();

        let x_id = h.find_by_name("X")[0].id().clone();
        let b_id = h.find_by_name("B")[0].id().clone();
        let a_id = h.find_by_name("A")[0].id().clone();

        t.reparent(h.model_mut(), &x_id, &b_id);

        assert!(t.is_dirty(&x_id));
        assert!(t.is_dirty(&a_id), "old parent should be dirty");
        assert!(t.is_dirty(&b_id), "new parent should be dirty");

        // X should now be under B
        let b = h.view(&b_id).unwrap();
        assert!(b.owned_members().iter().any(|m| m.name() == Some("X")));

        // X should not be under A
        let a = h.view(&a_id).unwrap();
        assert!(!a.owned_members().iter().any(|m| m.name() == Some("X")));
    }

    #[test]
    fn tracker_clear() {
        let mut h = host("package P { part def A; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        t.rename(h.model_mut(), &a_id, "B");
        assert!(t.has_changes());

        t.clear();
        assert!(!t.has_changes());
        assert!(!t.is_dirty(&a_id));
    }

    #[test]
    fn tracker_set_documentation() {
        let mut h = host("package P { part def A; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        t.set_documentation(h.model_mut(), &a_id, Some("A is great"));

        assert!(t.is_dirty(&a_id));
        let a_view = h.view(&a_id).unwrap();
        assert_eq!(a_view.element.documentation.as_deref(), Some("A is great"));
    }

    #[test]
    fn tracker_add_relationship() {
        let mut h = host("package P { part def Base; part def Derived; }");
        let mut t = h.tracker();

        let base_id = h.find_by_name("Base")[0].id().clone();
        let derived_id = h.find_by_name("Derived")[0].id().clone();

        t.add_relationship(
            h.model_mut(),
            ElementId::generate(),
            ElementKind::Specialization,
            derived_id.clone(),
            base_id.clone(),
        );

        assert!(t.is_dirty(&derived_id));
        assert!(h.model().relationship_count() > 0);

        // Check the relationship exists via element-based query
        let rels: Vec<_> = h.model().rel_elements_from(&derived_id).collect();
        let has_spec = rels
            .iter()
            .any(|r| r.kind == ElementKind::Specialization && r.target() == Some(&base_id));
        assert!(has_spec, "should have specialization relationship");
    }

    #[test]
    fn tracker_render_after_mutation() {
        let mut h = host("package P { part def Vehicle; }");
        let mut t = h.tracker();

        let v_id = h.find_by_name("Vehicle")[0].id().clone();
        t.rename(h.model_mut(), &v_id, "Car");

        let rendered = h.render();
        assert!(
            rendered.contains("Car"),
            "rendered text should contain renamed element: {rendered}"
        );
        assert!(
            !rendered.contains("Vehicle"),
            "rendered text should not contain old name: {rendered}"
        );
    }
}
