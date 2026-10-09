//! Which one, and under what heading: the two things that tell apart elements that read the
//! same. Two "Download" buttons have the same role, name and often the same classes; an agent
//! given only those changes both. "2nd of 2 on the page, under heading …" is what a person
//! would add.
//!
//! Finding them means looking through the whole page or window, so it is done once, for a
//! click, never for a hover, and within a budget. When the budget runs out nothing is said:
//! a count that stopped early would be a wrong count.

use super::Heading;
use std::time::{Duration, Instant};

/// How far the look through a window may go.
pub struct Budget {
    pub nodes: usize,
    pub time: Duration,
}

/// What a click may spend. The comment box waits for this, so it stays well under the time a
/// pause is noticed.
pub const BUDGET: Budget = Budget { nodes: 3000, time: Duration::from_millis(80) };

/// Headings are cut to about this many characters.
const HEADING_LENGTH: usize = 60;

/// One element met on the way through a page or window, in document order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Seen {
    /// It has the role and the name of the element that was picked.
    pub same: bool,
    /// It is the element that was picked.
    pub target: bool,
    /// It is a heading, and this is what it says.
    pub heading: Option<String>,
    /// It sits inside the element that was picked.
    pub inside: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Located {
    /// Which of the elements with its role and name it is, and how many there are. `None`
    /// when it is the only one.
    pub occurrence: Option<(u32, u32)>,
    pub heading: Option<Heading>,
}

/// "1st", "2nd", "3rd", "4th", "11th", "21st".
pub fn ordinal(n: u32) -> String {
    let ending = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{ending}")
}

/// A heading's text on one line, cut to about sixty characters.
pub fn heading_text(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= HEADING_LENGTH {
        return text;
    }
    format!("{}…", text.chars().take(HEADING_LENGTH - 1).collect::<String>().trim_end())
}

/// Works out the two facts from everything met, in document order. `None` when the picked
/// element was not among it: then nothing can be said.
///
/// The heading is the last one before the element. An element with none before it that holds
/// one itself (a section that begins with its own heading) is said to be "with" that one. A
/// heading that was picked is under no heading: its own words already say where it is.
pub fn place(seen: impl IntoIterator<Item = Seen>) -> Option<Located> {
    let (mut count, mut nth) = (0u32, None::<u32>);
    let (mut before, mut own) = (None::<String>, None::<String>);
    let (mut found, mut is_heading) = (false, false);
    for s in seen {
        if s.same {
            count += 1;
        }
        if s.target && !found {
            found = true;
            is_heading = s.heading.is_some();
            nth = s.same.then_some(count);
            continue;
        }
        match s.heading.filter(|text| !text.trim().is_empty()) {
            Some(text) if !found => before = Some(text),
            Some(text) if s.inside && own.is_none() => own = Some(text),
            _ => {}
        }
    }
    if !found {
        return None;
    }
    let heading = match (is_heading, before, own) {
        (true, _, _) => None,
        (_, Some(text), _) => Some(Heading { text: heading_text(&text), inside: false }),
        (_, None, Some(text)) => Some(Heading { text: heading_text(&text), inside: true }),
        _ => None,
    };
    Some(Located { occurrence: nth.filter(|_| count > 1).map(|nth| (nth, count)), heading })
}

/// What one question to the app says about a node.
pub struct Opened<N> {
    pub role: String,
    pub name: String,
    /// What it says, if it is a heading.
    pub heading: Option<String>,
    pub children: Vec<N>,
}

/// A node of an app's tree of elements. Asking is a call into the other app, so everything
/// about a node comes from one question.
pub trait Node: Sized {
    fn open(&self) -> Opened<Self>;
    /// Whether this is the same element as `other`.
    fn is(&self, other: &Self) -> bool;
}

/// Goes through the tree under `root` in document order, looking for `target` and for what
/// has its `role` and `name`. `None` when the budget ran out first, or `target` is not there.
pub fn walk<N: Node>(root: N, target: &N, role: &str, name: &str, budget: &Budget) -> Option<Located> {
    let started = Instant::now();
    let mut seen: Vec<Seen> = Vec::new();
    let mut stack = vec![(root, 0usize)];
    // How deep the picked element is, once met, and whether everything inside it has been seen.
    let (mut target_depth, mut left) = (None::<usize>, false);
    let (mut before, mut own) = (false, false);
    while let Some((node, depth)) = stack.pop() {
        if seen.len() >= budget.nodes || started.elapsed() > budget.time {
            return None;
        }
        let is_target = target_depth.is_none() && node.is(target);
        if target_depth.is_some_and(|d| depth <= d) {
            left = true;
        }
        let inside = target_depth.is_some() && !left;
        let opened = node.open();
        if is_target {
            target_depth = Some(depth);
        } else if opened.heading.is_some() {
            before |= target_depth.is_none();
            own |= inside;
        }
        seen.push(Seen { same: !name.is_empty() && opened.role == role && opened.name == name, target: is_target, heading: opened.heading, inside });
        // Without a name there is nothing to count: the heading is all that is looked for.
        if name.is_empty() && target_depth.is_some() && (before || own || left) {
            break;
        }
        stack.extend(opened.children.into_iter().rev().map(|child| (child, depth + 1)));
    }
    place(seen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn ordinals_read_the_way_people_say_them() {
        let said: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 22, 23, 101, 111, 112].into_iter().map(ordinal).collect();
        assert_eq!(said, ["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "23rd", "101st", "111th", "112th"]);
    }

    #[test]
    fn a_long_heading_is_cut_and_put_on_one_line() {
        assert_eq!(heading_text("  Try it on\n your own app. "), "Try it on your own app.");
        let long = heading_text(&"word ".repeat(30));
        assert_eq!(long.chars().count(), 60);
        assert!(long.ends_with("word…"), "{long}");
    }

    /// A page as a test writes it: (id, role, name, children). A role of "Heading" is one.
    #[derive(Clone)]
    struct Fake {
        id: u32,
        role: &'static str,
        name: &'static str,
        children: Vec<Fake>,
        opened: Rc<Cell<usize>>,
    }

    impl Node for Fake {
        fn open(&self) -> Opened<Fake> {
            self.opened.set(self.opened.get() + 1);
            Opened { role: self.role.into(), name: self.name.into(), heading: (self.role == "Heading").then(|| self.name.to_string()), children: self.children.clone() }
        }

        fn is(&self, other: &Fake) -> bool {
            self.id == other.id
        }
    }

    struct Page {
        root: Fake,
        opened: Rc<Cell<usize>>,
    }

    impl Page {
        fn find(&self, id: u32) -> Fake {
            fn look(node: &Fake, id: u32) -> Option<Fake> {
                if node.id == id { Some(node.clone()) } else { node.children.iter().find_map(|c| look(c, id)) }
            }
            look(&self.root, id).expect("an element with that id")
        }

        fn locate(&self, id: u32, budget: &Budget) -> Option<Located> {
            let target = self.find(id);
            walk(self.root.clone(), &target, target.role, target.name, budget)
        }
    }

    /// The website: a top bar with a download link, a section with its own heading and the
    /// same link again, and a questions section.
    fn site() -> Page {
        let opened = Rc::new(Cell::new(0));
        let n = |id, role, name, children| Fake { id, role, name, children, opened: opened.clone() };
        let root = n(1, "WebArea", "", vec![
            n(2, "Group", "", vec![n(3, "Link", "Download for Windows", vec![]), n(4, "Link", "GitHub", vec![])]),
            n(5, "Heading", "Point at what you want changed.", vec![]),
            n(6, "Group", "", vec![
                n(7, "Heading", "Try it on your own app.", vec![]),
                n(8, "Link", "Download for Windows", vec![n(9, "StaticText", "Download for Windows", vec![])]),
                n(10, "Button", "Download for Windows", vec![]),
            ]),
            n(11, "Group", "", vec![n(12, "Heading", "Questions", vec![]), n(13, "Link", "GitHub", vec![]), n(14, "Group", "", vec![])]),
        ]);
        Page { root, opened }
    }

    const ROOMY: Budget = Budget { nodes: 1000, time: Duration::from_secs(5) };

    fn under(text: &str) -> Option<Heading> {
        Some(Heading { text: text.into(), inside: false })
    }

    #[test]
    fn the_second_of_two_identical_links_is_told_apart_by_its_place_and_its_heading() {
        let page = site();
        assert_eq!(page.locate(8, &ROOMY), Some(Located { occurrence: Some((2, 2)), heading: under("Try it on your own app.") }));
        // The first one is above every heading. Same name, other role: not counted.
        assert_eq!(page.locate(3, &ROOMY), Some(Located { occurrence: Some((1, 2)), heading: None }));
        assert_eq!(page.locate(13, &ROOMY), Some(Located { occurrence: Some((2, 2)), heading: under("Questions") }));
    }

    #[test]
    fn something_that_is_the_only_one_of_its_kind_has_no_number() {
        let page = site();
        assert_eq!(page.locate(10, &ROOMY), Some(Located { occurrence: None, heading: under("Try it on your own app.") }));
    }

    #[test]
    fn a_section_is_named_by_the_heading_before_it_or_else_the_one_it_holds() {
        let page = site();
        // The top bar: nothing before it, nothing in it.
        assert_eq!(page.locate(2, &ROOMY), Some(Located::default()));
        assert_eq!(page.locate(6, &ROOMY), Some(Located { occurrence: None, heading: under("Point at what you want changed.") }));
        // The same sections on a page with no heading above them.
        let opened = Rc::new(Cell::new(0));
        let n = |id, role, name, children| Fake { id, role, name, children, opened: opened.clone() };
        let root = n(1, "WebArea", "", vec![n(2, "Group", "", vec![n(3, "Group", "", vec![n(4, "Heading", "Questions", vec![])])]), n(5, "Group", "", vec![n(6, "Heading", "Later", vec![])])]);
        let page = Page { root, opened };
        assert_eq!(page.locate(2, &ROOMY), Some(Located { occurrence: None, heading: Some(Heading { text: "Questions".into(), inside: true }) }));
        // An empty group gets nothing from a heading that only comes after it.
        let opened = Rc::new(Cell::new(0));
        let n = |id, role, name, children| Fake { id, role, name, children, opened: opened.clone() };
        let page = Page { root: n(1, "WebArea", "", vec![n(2, "Group", "", vec![]), n(3, "Heading", "Later", vec![])]), opened };
        assert_eq!(page.locate(2, &ROOMY), Some(Located::default()));
    }

    #[test]
    fn a_heading_that_was_picked_is_not_said_to_be_under_another() {
        let page = site();
        assert_eq!(page.locate(12, &ROOMY), Some(Located::default()));
    }

    #[test]
    fn something_without_a_name_is_not_counted_and_the_look_stops_early() {
        let page = site();
        assert_eq!(page.locate(6, &ROOMY).unwrap().occurrence, None, "three unnamed groups, and no number");
        assert!(page.opened.get() < 8, "the questions section was never asked about: {} asked", page.opened.get());
    }

    #[test]
    fn when_the_budget_runs_out_nothing_is_said() {
        let page = site();
        // Enough for the first link, but not to know how many there are.
        assert_eq!(page.locate(3, &Budget { nodes: 5, time: Duration::from_secs(5) }), None);
        assert_eq!(page.opened.get(), 5, "and it stopped asking there");
        assert_eq!(page.locate(8, &Budget { nodes: 1000, time: Duration::ZERO }), None, "no time at all");
        assert_eq!(page.locate(8, &Budget { nodes: 14, time: Duration::from_secs(5) }).unwrap().occurrence, Some((2, 2)), "exactly enough");
    }

    #[test]
    fn an_element_that_is_not_in_the_tree_gets_nothing() {
        let page = site();
        let stranger = Fake { id: 99, role: "Link", name: "GitHub", children: vec![], opened: page.opened.clone() };
        assert_eq!(walk(page.root.clone(), &stranger, "Link", "GitHub", &ROOMY), None);
    }

    #[test]
    fn a_flat_list_in_document_order_gives_the_same_answers() {
        // What Windows hands over: only the headings and the look-alikes, no tree.
        let heading = |text: &str| Seen { heading: Some(text.into()), ..Default::default() };
        let same = Seen { same: true, ..Default::default() };
        let target = Seen { same: true, target: true, ..Default::default() };
        let seen = vec![same.clone(), heading("Invoices"), same.clone(), target.clone(), heading("Later"), same.clone()];
        assert_eq!(place(seen), Some(Located { occurrence: Some((3, 4)), heading: under("Invoices") }));
        assert_eq!(place(vec![same, heading("Invoices")]), None, "the picked element was not in the list");
        let own = Seen { heading: Some("Questions".into()), inside: true, ..Default::default() };
        assert_eq!(place(vec![Seen { target: true, ..Default::default() }, own]).unwrap().heading, Some(Heading { text: "Questions".into(), inside: true }));
    }
}
