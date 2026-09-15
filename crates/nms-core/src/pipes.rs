//! Which machines at a base share a supply network, worked out from where its pipes were laid.
//!
//! A base object records no network membership, but the pipes themselves carry the plumbing. On an ordinary object `At` is a unit orientation vector; on a line part (`^U_PIPELINE`, `^U_POWERLINE`, `^U_PORTALLINE`) it is the run itself, so the part spans `position` to `position + at`, and its far end overshoots the joint it meets by exactly one unit. Joining segments whose corrected ends coincide, attaching a machine whose own connector distance is met, and chaining machines that stand touching, reproduces the networks the game shows.
//!
//! Nothing here reads the save: callers hand in segments and machines, so the rules are testable on their own.

/// Two ends within this distance are the same joint. Real joints land within a thousandth of a unit and the nearest pair that is not a joint is most of a unit away, so the figure is not delicate.
const JOIN_TOLERANCE: f32 = 0.05;

/// Slack allowed on a machine's connector distance when deciding whether a line ends at it.
const CONNECTOR_TOLERANCE: f32 = 0.05;

/// Machines standing at least this close share a network with no pipe between them. Depots seen touching sit 1.40 to 1.83 apart and the next pair up is 2.13.
pub const TOUCH_DISTANCE: f32 = 2.0;

/// How far from a supply depot's own position a pipe meets it.
pub const DEPOT_CONNECTOR: f32 = 1.032;

/// How far from an extractor's own position a pipe meets it. Mineral and gas extractors are the same.
pub const EXTRACTOR_CONNECTOR: f32 = 1.069;

/// One laid line part, with the one-unit overshoot at its far end already taken off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub start: [f32; 3],
    pub end: [f32; 3],
}

impl Segment {
    /// The run a line part describes, or `None` when `at` is a unit orientation vector rather than a run.
    pub fn run(position: [f32; 3], at: [f32; 3]) -> Option<Self> {
        let length = length(at);
        if !length.is_finite() || length <= 1.0 {
            return None;
        }
        let mut end = [0.0; 3];
        for axis in 0..3 {
            end[axis] = position[axis] + at[axis] - at[axis] / length;
        }
        Some(Self {
            start: position,
            end,
        })
    }

    fn ends(&self) -> [[f32; 3]; 2] {
        [self.start, self.end]
    }
}

/// A machine a line can end at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attachment {
    pub position: [f32; 3],
    /// How far from `position` a line meets this kind of machine.
    pub connector: f32,
}

/// Group machines into networks, numbering them from 1 in the order the machines are given.
///
/// A machine attached to nothing is a network of its own, which is what the game shows for a depot with no pipe.
pub fn networks(segments: &[Segment], machines: &[Attachment]) -> Vec<u32> {
    let mut sets = DisjointSets::new(segments.len() + machines.len());
    let machine = |index: usize| segments.len() + index;

    for (a, first) in segments.iter().enumerate() {
        for (b, second) in segments.iter().enumerate().skip(a + 1) {
            if first.ends().iter().any(|one| {
                second
                    .ends()
                    .iter()
                    .any(|two| within(*one, *two, JOIN_TOLERANCE))
            }) {
                sets.union(a, b);
            }
        }
    }

    for (index, attachment) in machines.iter().enumerate() {
        for (segment_index, segment) in segments.iter().enumerate() {
            let meets = segment.ends().iter().any(|end| {
                (distance(*end, attachment.position) - attachment.connector).abs()
                    < CONNECTOR_TOLERANCE
            });
            if meets {
                sets.union(machine(index), segment_index);
            }
        }
    }

    for (a, first) in machines.iter().enumerate() {
        for (b, second) in machines.iter().enumerate().skip(a + 1) {
            if within(first.position, second.position, TOUCH_DISTANCE) {
                sets.union(machine(a), machine(b));
            }
        }
    }

    let mut numbers = Vec::with_capacity(machines.len());
    let mut seen: Vec<(usize, u32)> = Vec::new();
    for index in 0..machines.len() {
        let root = sets.find(machine(index));
        let number = match seen.iter().find(|(r, _)| *r == root) {
            Some((_, number)) => *number,
            None => {
                let number = seen.len() as u32 + 1;
                seen.push((root, number));
                number
            }
        };
        numbers.push(number);
    }
    numbers
}

/// A network's letter: 1 is `A`, 27 is `AA`. Networks are numbered, not named, in the save; the letters are only so a base's networks can be told apart when they are listed.
pub fn label(number: u32) -> String {
    if number == 0 {
        return "-".to_string();
    }
    let mut remaining = number;
    let mut out = Vec::new();
    while remaining > 0 {
        let step = (remaining - 1) % 26;
        out.push((b'A' + step as u8) as char);
        remaining = (remaining - 1) / 26;
    }
    out.iter().rev().collect()
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    length([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
}

fn within(a: [f32; 3], b: [f32; 3], limit: f32) -> bool {
    distance(a, b) < limit
}

/// Union-find over segments then machines.
struct DisjointSets {
    parent: Vec<usize>,
}

impl DisjointSets {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    fn find(&mut self, mut index: usize) -> usize {
        while self.parent[index] != index {
            self.parent[index] = self.parent[self.parent[index]];
            index = self.parent[index];
        }
        index
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.parent[a] = b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pipe laid from `from` to `to`, as the save would record it.
    fn pipe(from: [f32; 3], to: [f32; 3]) -> Segment {
        let run = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let len = length(run);
        let at = [
            run[0] + run[0] / len,
            run[1] + run[1] / len,
            run[2] + run[2] / len,
        ];
        Segment::run(from, at).expect("a run longer than a unit")
    }

    fn depot(position: [f32; 3]) -> Attachment {
        Attachment {
            position,
            connector: DEPOT_CONNECTOR,
        }
    }

    fn extractor(position: [f32; 3]) -> Attachment {
        Attachment {
            position,
            connector: EXTRACTOR_CONNECTOR,
        }
    }

    #[test]
    fn test_a_unit_vector_is_an_orientation_not_a_run() {
        assert_eq!(Segment::run([1.0, 2.0, 3.0], [0.0, 0.0, 1.0]), None);
        assert_eq!(Segment::run([0.0; 3], [0.0; 3]), None);
        assert!(Segment::run([0.0; 3], [0.0, 0.0, 4.0]).is_some());
    }

    #[test]
    fn test_the_far_end_drops_the_one_unit_overshoot() {
        let segment = Segment::run([0.0; 3], [0.0, 0.0, 5.0]).unwrap();
        assert_eq!(segment.start, [0.0; 3]);
        assert_eq!(segment.end, [0.0, 0.0, 4.0]);
    }

    #[test]
    fn test_a_pipe_between_two_machines_makes_one_network() {
        let one = extractor([0.0; 3]);
        let other = depot([20.0, 0.0, 0.0]);
        let run = pipe(
            [EXTRACTOR_CONNECTOR, 0.0, 0.0],
            [20.0 - DEPOT_CONNECTOR, 0.0, 0.0],
        );
        assert_eq!(networks(&[run], &[one, other]), vec![1, 1]);
    }

    #[test]
    fn test_machines_with_no_pipe_are_networks_of_their_own() {
        let machines = [depot([0.0; 3]), depot([20.0, 0.0, 0.0])];
        assert_eq!(networks(&[], &machines), vec![1, 2]);
    }

    #[test]
    fn test_touching_machines_chain_without_a_pipe() {
        let close = [depot([0.0; 3]), depot([1.5, 0.0, 0.0])];
        assert_eq!(networks(&[], &close), vec![1, 1]);
        let apart = [depot([0.0; 3]), depot([2.5, 0.0, 0.0])];
        assert_eq!(networks(&[], &apart), vec![1, 2]);
    }

    #[test]
    fn test_a_pipe_passing_near_a_machine_does_not_attach_to_it() {
        // The run ends a long way from the bystander, and half a unit short of the far depot's connector.
        let bystander = depot([10.0, 5.0, 0.0]);
        let target = depot([20.0, 0.0, 0.0]);
        let run = pipe([0.0; 3], [20.0 - DEPOT_CONNECTOR - 0.5, 0.0, 0.0]);
        assert_eq!(
            networks(&[run], &[bystander, target]),
            vec![1, 2],
            "neither machine meets the run at its own connector distance"
        );
    }

    #[test]
    fn test_two_runs_meeting_end_to_end_are_one_network() {
        let first = depot([0.0; 3]);
        let second = depot([30.0, 0.0, 0.0]);
        let corner = [15.0, 0.0, 0.0];
        let a = pipe([DEPOT_CONNECTOR, 0.0, 0.0], corner);
        let b = pipe(corner, [30.0 - DEPOT_CONNECTOR, 0.0, 0.0]);
        assert_eq!(networks(&[a, b], &[first, second]), vec![1, 1]);
    }

    #[test]
    fn test_networks_are_numbered_in_the_order_the_machines_are_given() {
        let machines = [
            depot([0.0; 3]),
            depot([40.0, 0.0, 0.0]),
            depot([0.5, 0.0, 0.0]),
        ];
        assert_eq!(networks(&[], &machines), vec![1, 2, 1]);
    }

    #[test]
    fn test_labels_count_in_letters() {
        assert_eq!(label(1), "A");
        assert_eq!(label(2), "B");
        assert_eq!(label(26), "Z");
        assert_eq!(label(27), "AA");
        assert_eq!(label(28), "AB");
        assert_eq!(label(0), "-");
    }
}
