//! A small ordered frame graph (docs/specs/RENDER.md R21.1).
//!
//! Passes name the resources they read and write; a pass may only read what an earlier pass
//! wrote and only one pass may write a resource. The engine registers its passes here so a
//! missing or mis-ordered target is a test failure rather than a validation error on the GPU.

/// One pass of the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    pub name: String,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

/// Why a graph cannot run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    #[error("the frame graph has no passes")]
    Empty,
    #[error("pass {0:?} is already in the graph")]
    DuplicatePass(String),
    #[error("pass {0:?} reads {1:?}, which no earlier pass writes")]
    NotYetWritten(String, String),
    #[error("pass {0:?} and an earlier pass both write {1:?}")]
    TwoWriters(String, String),
}

/// The ordered passes of one frame.
#[derive(Debug, Clone, Default)]
pub struct FrameGraph {
    passes: Vec<Pass>,
    inputs: Vec<String>,
}

impl FrameGraph {
    pub fn new() -> FrameGraph {
        FrameGraph::default()
    }

    /// Declares a resource the host already uploaded (the triangle buffer), so the first pass
    /// may read it.
    pub fn input(&mut self, name: &str) {
        self.inputs.push(name.to_string());
    }

    /// Appends a pass. `reads` and `writes` are resource names such as `g0` or `shadow:0`.
    pub fn add(&mut self, name: &str, reads: &[&str], writes: &[&str]) {
        self.passes.push(Pass {
            name: name.to_string(),
            reads: reads.iter().map(|s| s.to_string()).collect(),
            writes: writes.iter().map(|s| s.to_string()).collect(),
        });
    }

    pub fn passes(&self) -> &[Pass] {
        &self.passes
    }

    pub fn len(&self) -> usize {
        self.passes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }

    /// Pass order, one writer per resource and no read before its writer.
    pub fn validate(&self) -> Result<(), GraphError> {
        if self.passes.is_empty() {
            return Err(GraphError::Empty);
        }
        let mut names: Vec<&str> = Vec::new();
        let mut written: Vec<&str> = self.inputs.iter().map(|s| s.as_str()).collect();
        for p in &self.passes {
            if names.contains(&p.name.as_str()) {
                return Err(GraphError::DuplicatePass(p.name.clone()));
            }
            names.push(&p.name);
            for r in &p.reads {
                if !written.contains(&r.as_str()) {
                    return Err(GraphError::NotYetWritten(p.name.clone(), r.clone()));
                }
            }
            for w in &p.writes {
                if written.contains(&w.as_str()) {
                    return Err(GraphError::TwoWriters(p.name.clone(), w.clone()));
                }
                written.push(w);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_shape_validates() {
        let mut g = FrameGraph::new();
        g.input("tris");
        g.add("shadow:0", &["tris"], &["shadow:0"]);
        g.add(
            "gbuffer",
            &["tris"],
            &["g0", "g1", "g2", "g3", "g4", "depth"],
        );
        g.add(
            "lighting",
            &["g0", "g1", "g2", "g3", "g4", "depth", "shadow:0"],
            &["beauty", "diffuse", "specular", "shadow"],
        );
        assert_eq!(g.validate(), Ok(()));
        assert_eq!(g.len(), 3);
    }

    #[test]
    fn an_empty_graph_is_rejected() {
        assert_eq!(FrameGraph::new().validate(), Err(GraphError::Empty));
    }

    #[test]
    fn a_read_before_any_writer_is_rejected() {
        let mut g = FrameGraph::new();
        g.add("lighting", &["g0"], &["beauty"]);
        assert_eq!(
            g.validate(),
            Err(GraphError::NotYetWritten("lighting".into(), "g0".into()))
        );
    }

    #[test]
    fn two_writers_are_rejected() {
        let mut g = FrameGraph::new();
        g.add("a", &[], &["g0"]);
        g.add("b", &["g0"], &["g0"]);
        assert_eq!(
            g.validate(),
            Err(GraphError::TwoWriters("b".into(), "g0".into()))
        );
    }

    #[test]
    fn a_repeated_pass_name_is_rejected() {
        let mut g = FrameGraph::new();
        g.add("a", &[], &["x"]);
        g.add("a", &[], &["y"]);
        assert_eq!(g.validate(), Err(GraphError::DuplicatePass("a".into())));
    }
}
