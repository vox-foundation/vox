//! Default achievement definitions (split for file size).

mod doubt;
mod part_a;
mod part_b;
mod part_c;

use crate::achievement::Achievement;

pub(super) fn all() -> Vec<Achievement> {
    let mut v = part_a::part_a();
    v.extend(part_b::part_b());
    v.extend(part_c::part_c());
    v.extend(doubt::doubt_achievements());
    v
}
