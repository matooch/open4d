//! Clone data: the per-clone arrays the cloner produces, effectors modify and consumers
//! read (clone-toolset spec C1).

use glam::{DMat4, DVec3};

/// One clone before it is appended to a [`CloneData`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloneItem {
    pub matrix: DMat4,
    pub color: [f64; 3],
    pub weight: f64,
    pub visible: bool,
    pub uvw: DVec3,
    pub time: f64,
}

impl CloneItem {
    /// A visible white clone with the given matrix.
    pub fn at(matrix: DMat4, uvw: DVec3) -> Self {
        CloneItem {
            matrix,
            color: [1.0; 3],
            weight: 0.0,
            visible: true,
            uvw,
            time: 0.0,
        }
    }
}

/// Per-clone arrays (struct of arrays); every vector has the same length. Cheap to copy:
/// effectors take it by `&mut`, snapshots are `clone()`s.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CloneData {
    /// Clone matrices in cloner space (columns: X, Y, Z axis with scale, position).
    pub matrices: Vec<DMat4>,
    pub colors: Vec<[f64; 3]>,
    pub weights: Vec<f64>,
    /// The clone-on flag. Hidden clones keep their slot.
    pub visible: Vec<bool>,
    /// Position of the clone in its distribution, 0..1 per axis.
    pub uvw: Vec<DVec3>,
    /// Clone index (position in the list).
    pub index: Vec<u32>,
    /// Per-clone time offset in seconds.
    pub time: Vec<f64>,
    /// The prototype (child of the cloner) each clone shows.
    pub child: Vec<u32>,
}

impl CloneData {
    pub fn len(&self) -> usize {
        self.matrices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matrices.is_empty()
    }

    /// Appends a clone; its index is the new position and its prototype 0.
    pub fn push(&mut self, item: CloneItem) {
        self.index.push(self.matrices.len() as u32);
        self.matrices.push(item.matrix);
        self.colors.push(item.color);
        self.weights.push(item.weight);
        self.visible.push(item.visible);
        self.uvw.push(item.uvw);
        self.time.push(item.time);
        self.child.push(0);
    }

    /// Number of clones with the visible flag on.
    pub fn visible_count(&self) -> usize {
        self.visible.iter().filter(|v| **v).count()
    }

    /// True if all arrays have the length of `matrices`.
    pub fn is_consistent(&self) -> bool {
        let n = self.matrices.len();
        [
            self.colors.len(),
            self.weights.len(),
            self.visible.len(),
            self.uvw.len(),
            self.index.len(),
            self.time.len(),
            self.child.len(),
        ]
        .iter()
        .all(|l| *l == n)
    }
}
