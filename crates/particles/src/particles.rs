//! The struct-of-arrays particle data (DYNAMICS.md section 9.6).

use glam::DVec3;

use crate::emitter::Newborn;

/// The particles of one frame, world space. Every vector has one entry per particle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Particles {
    /// Emission index, counted from 0 over the whole run.
    pub id: Vec<u64>,
    pub position: Vec<DVec3>,
    pub velocity: Vec<DVec3>,
    /// Seconds since birth.
    pub age: Vec<f64>,
    /// Seconds.
    pub lifetime: Vec<f64>,
    /// True for every entry of a frame snapshot; false only between a deflector kill and the
    /// removal of the particle.
    pub alive: Vec<bool>,
    pub scale: Vec<f64>,
    /// Spin angle in radians.
    pub rotation: Vec<f64>,
    pub birth_frame: Vec<i64>,
    pub spin_axis: Vec<DVec3>,
    /// Radians per second.
    pub spin_rate: Vec<f64>,
    pub scale_end: Vec<f64>,
}

fn compact<T>(v: &mut Vec<T>, keep: &[bool]) {
    let mut i = 0;
    v.retain(|_| {
        let k = keep[i];
        i += 1;
        k
    });
}

impl Particles {
    pub fn len(&self) -> usize {
        self.id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    /// Entries whose `alive` flag is set.
    pub fn alive_count(&self) -> usize {
        self.alive.iter().filter(|a| **a).count()
    }

    pub(crate) fn push(&mut self, id: u64, frame: i64, b: &Newborn) {
        self.id.push(id);
        self.position.push(b.position);
        self.velocity.push(b.velocity);
        self.age.push(0.0);
        self.lifetime.push(b.lifetime);
        self.alive.push(true);
        self.scale.push(1.0);
        self.rotation.push(0.0);
        self.birth_frame.push(frame);
        self.spin_axis.push(b.spin_axis);
        self.spin_rate.push(b.spin_rate);
        self.scale_end.push(b.scale_end);
    }

    /// Removes the entries that are not alive.
    pub(crate) fn remove_dead(&mut self) {
        let keep = self.alive.clone();
        if keep.iter().all(|k| *k) {
            return;
        }
        compact(&mut self.id, &keep);
        compact(&mut self.position, &keep);
        compact(&mut self.velocity, &keep);
        compact(&mut self.age, &keep);
        compact(&mut self.lifetime, &keep);
        compact(&mut self.alive, &keep);
        compact(&mut self.scale, &keep);
        compact(&mut self.rotation, &keep);
        compact(&mut self.birth_frame, &keep);
        compact(&mut self.spin_axis, &keep);
        compact(&mut self.spin_rate, &keep);
        compact(&mut self.scale_end, &keep);
    }
}
