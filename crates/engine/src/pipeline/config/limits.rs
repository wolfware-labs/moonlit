//! Bounds on the YAML tree built from a release file.
//!
//! The reader refuses a collection nested past [`MAX_DEPTH`] at its opening event, before it reads
//! on, and the tree builder charges each alias before it creates the alias node, so a file that goes
//! past a limit is refused before the work it asks for is done. Because the reader runs first,
//! nesting past the depth limit is reported before alias-budget errors, wherever each occurs.
//!
//! Only the first document of the file is the release file. The reader keeps that document's events
//! and reads the documents after it for syntax errors alone: it keeps none of their events and checks
//! neither limit in them.

/// Deepest nesting of collections (sequences and mappings) a release file may use. The root
/// mapping is level 1. Every later stage walks the tree recursively, so this also bounds their
/// stack use.
pub(super) const MAX_DEPTH: usize = 64;

/// Most nodes that aliases may expand to in one document, summed over every alias in it.
pub(super) const MAX_ALIAS_NODES: usize = 100_000;

/// Most scalar bytes (as stored, `str::len`) that aliases may expand to in one document, summed over
/// every alias in it.
pub(super) const MAX_ALIAS_BYTES: usize = 8 << 20;

/// What a built node costs once every alias inside it is expanded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Size {
  /// Nodes in the subtree, the node itself included.
  pub nodes: usize,
  /// Scalar bytes in the subtree.
  pub bytes: usize,
  /// Levels of collections in the subtree: 0 for a scalar, 1 for a collection of scalars.
  pub height: usize,
}

impl Size {
  #[must_use]
  pub fn scalar(bytes: usize) -> Self {
    Size {
      nodes: 1,
      bytes,
      height: 0,
    }
  }

  #[must_use]
  pub fn empty_collection() -> Self {
    Size {
      nodes: 1,
      bytes: 0,
      height: 1,
    }
  }

  /// Adds one child of a collection.
  pub fn add_child(&mut self, child: Size) {
    self.nodes = self.nodes.saturating_add(child.nodes);
    self.bytes = self.bytes.saturating_add(child.bytes);
    self.height = self.height.max(child.height.saturating_add(1));
  }
}

/// Whether a subtree of `height` levels, placed under `enclosing` collections, nests past
/// [`MAX_DEPTH`].
#[must_use]
pub(super) fn too_deep(enclosing: usize, height: usize) -> bool {
  enclosing.saturating_add(height) > MAX_DEPTH
}

/// The running total of what aliases have expanded to in the document so far.
#[derive(Debug, Default)]
pub(super) struct AliasBudget {
  nodes: usize,
  bytes: usize,
}

impl AliasBudget {
  /// Charges one alias that expands to `size`. Returns `false`, and charges nothing, when that
  /// would take the total past [`MAX_ALIAS_NODES`] or [`MAX_ALIAS_BYTES`].
  #[must_use]
  pub fn charge(&mut self, size: Size) -> bool {
    let nodes = self.nodes.saturating_add(size.nodes);
    let bytes = self.bytes.saturating_add(size.bytes);
    if nodes > MAX_ALIAS_NODES || bytes > MAX_ALIAS_BYTES {
      return false;
    }
    self.nodes = nodes;
    self.bytes = bytes;
    true
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn collection_size_sums_children_and_takes_the_tallest() {
    let mut inner = Size::empty_collection();
    inner.add_child(Size::scalar(3));
    let mut outer = Size::empty_collection();
    outer.add_child(inner);
    outer.add_child(Size::scalar(4));
    assert_eq!(
      outer,
      Size {
        nodes: 4,
        bytes: 7,
        height: 2
      }
    );
  }

  #[test]
  fn depth_limit_is_inclusive() {
    assert!(!too_deep(MAX_DEPTH - 1, 1));
    assert!(too_deep(MAX_DEPTH, 1));
    assert!(!too_deep(MAX_DEPTH, 0));
  }

  #[test]
  fn budget_allows_exactly_the_limits() {
    let mut budget = AliasBudget::default();
    assert!(budget.charge(Size {
      nodes: MAX_ALIAS_NODES,
      bytes: MAX_ALIAS_BYTES,
      height: 0
    }));
    assert!(!budget.charge(Size::scalar(0)));
  }

  #[test]
  fn refused_charge_leaves_the_total_unchanged() {
    let mut budget = AliasBudget::default();
    assert!(!budget.charge(Size::scalar(MAX_ALIAS_BYTES + 1)));
    assert!(budget.charge(Size::scalar(MAX_ALIAS_BYTES)));
  }
}
