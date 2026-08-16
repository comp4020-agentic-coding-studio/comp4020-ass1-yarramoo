use crate::vector::V3;

/// An orthonormal basis oriented around a reference axis `w` (typically a
/// surface normal). `local` maps a direction expressed relative to that
/// basis's own z-axis (e.g. a cosine-weighted sample generated in "z-up"
/// space) into world space.
pub struct Onb {
    u: V3,
    v: V3,
    w: V3,
}

impl Onb {
    pub fn from_normal(n: V3) -> Self {
        let w = n.normalize();
        // Any axis not parallel to w works as a seed for the cross product;
        // picking whichever world axis w is *least* aligned with keeps the
        // cross product well-conditioned.
        let a = if w.x.abs() > 0.9 { V3::new(0., 1., 0.) } else { V3::new(1., 0., 0.) };
        let v = w.cross(&a).normalize();
        let u = w.cross(&v);
        Self { u, v, w }
    }

    pub fn local(&self, v: V3) -> V3 {
        v.x * self.u + v.y * self.v + v.z * self.w
    }

    pub fn w(&self) -> V3 {
        self.w
    }
}
