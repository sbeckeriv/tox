//#![deny(warnings)]

use crate::grammar::{Rule, Symbol};
use std::collections::HashSet;
use std::{cell, fmt, hash, iter};
use std::rc::Rc;


#[derive(PartialEq,Eq,Hash,Debug,Clone)]
pub enum Trigger {
    Complete(Rc<Item>),
    Scan(String),
}

/// An Item is a partially matched `Rule`. `dot` shows the match progress.
pub struct Item {
    pub rule: Rc<Rule>,  // LR0item (dotted rule)
    pub dot: usize,      // dot position within the rule
    pub start: usize,    // input stream position where item starts
    pub end: usize,      // input stream position where item ends

    // Need a RefCell to update existing Items. A replacement with the union
    // of backpointers would invalidate other Items already pointing to this one.
    // Those invalidated items wouldn't have the whole back-pointer list.
    /// backpointers leading to this item: (source-item, Scan/Complete)
    backpointers: cell::RefCell<HashSet<(Rc<Item>, Trigger)>>,
}


// Items are deduped only by rule, dot, start, end (ie: not bp)
// The intention is that 2 Items are the same and can be merged ignoring bp.
impl hash::Hash for Item {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.rule.hash(state);
        self.dot.hash(state);
        self.start.hash(state);
        self.end.hash(state);
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Item) -> bool {
        self.rule == other.rule &&
        self.dot == other.dot &&
        self.start == other.start &&
        self.end == other.end
    }
}

impl Eq for Item {}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pre = self.rule.spec.iter().take(self.dot)
            .map(|s| s.name()).collect::<Vec<_>>().join(" ");
        let post = self.rule.spec.iter().skip(self.dot)
            .map(|s| s.name()).collect::<Vec<_>>().join(" ");
        write!(f, "({} - {}) {} -> {} \u{00b7} {} #bp: {}",
               self.start, self.end, self.rule.head, pre, post,
               self.backpointers.borrow().len())
    }
}

impl Item {
    /// Item is complete if Rule has being fully matched
    pub fn complete(&self) -> bool {
        self.dot >= self.rule.spec.len()
    }

    /// Exposes the next symbol in the progress of the Rule
    pub fn next_symbol(&self) -> Option<&Symbol> {
        self.rule.spec.get(self.dot).map(|sym| &**sym)
    }

    /// Scans or Completions that led to the creation of this Item.
    /// only ever borrowed non-mutable ref returned for public consumption
    pub fn sources(&self) -> cell::Ref<HashSet<(Rc<Item>, Trigger)>> {
        self.backpointers.borrow()
    }

    ///
    pub fn merge_sources(&self, other: Item) {
        let other_bp = other.backpointers.into_inner();
        self.backpointers.borrow_mut().extend(other_bp);
    }

    /// Build a new `Prediction` based Item.
    pub fn predict_new(rule: &Rc<Rule>, start: usize) -> Item {
        Item{
            rule: rule.clone(),
            dot: 0,
            start,
            end: start,
            backpointers: cell::RefCell::new(HashSet::new()),
        }
    }

    /// Build `Scan` based Items.
    /// An item where the rule is advanced by matching a terminal.
    pub fn scan_new(source: &Rc<Item>, end: usize, input: &str) -> Item {
        let mut _bp = HashSet::new();
        _bp.insert((source.clone(), Trigger::Scan(input.to_string())));
        Item{
            rule: source.rule.clone(),
            dot: source.dot + 1,
            start: source.start,
            end,
            backpointers: cell::RefCell::new(_bp),
        }
    }

    /// Build `Completion` based Items.
    /// `Rule` is advanced because its next symbol matches the completed `trigger`.
    pub fn complete_new(source: &Rc<Item>, trigger: &Rc<Item>, end: usize) -> Item {
        let mut _bp = HashSet::new();
        _bp.insert((source.clone(), Trigger::Complete(trigger.clone())));
        Item{
            rule: source.rule.clone(),
            dot: source.dot + 1,
            start: source.start,
            end,
            backpointers: cell::RefCell::new(_bp),
        }
    }
}


//#[derive(Default)]
//pub struct StateSet(HashSet<Rc<Item>>);

// NOTE: only the current stateset ever needs updating via extend/insert
// This means that Items on previous statesets don't get modified.
// Only items in the current state set are updated ... can we drop the RefCell?
//
// IMPORTANT:?
// Can item bps in the current or next state reference items in the current stateset?
//
// - Scans in the next state always reference the current(preceding) state,
//   but are generated after the current state has stopped morphing. By then
//   we're no longer modifying the current stateset, so Si+1 items will only
//   be generated once all Si items are fixed. So we can replace/tweak them.
//
// - Item backpointers in the current state can reference the previous
//   (eg scans), current (eg: from previous predict/complete loop ones,
//   item generated by completion references the completion item in same state).
//
//   but are generated after the current state has stopped morphing. By then
//   we're no longer modifying the current stateset, so Si+1 items will only
//   be generated once all Si items are fixed. So we can replace/tweak them.
//
//   can there be a reference loop? item bp:completion -> item<=, no completion
//   's bp can't reference item that didn't exist by then.
//
//   Items are created referencing pre-existing ones so those can't reference
//   the one just being created.
//
//   S -> <e> can trigger a completion in current state of prediction in state,
//   eg: S -> S x, S -> <e>  .. S -> S . x
//
//   Predictions don't reference sources. Only scans and completions have them.
//   A completion can reference an item in same stateset which can later be modified?
//
//   What if someone was already referencing 17?
//
//   a, b -> c .. d

//impl StateSet {
    //// Add Earley Items into the set. If the Item already exists we merge bp.
    //// StateSets override insertion to merge back-pointers for existing Items.
    //// See implementations of Hash + PartialEq + Eq for Item excluding Item::bp
    //fn insert(&mut self, item: Item) {
        //if let Some(existent) = self.0.get(&item) {
            ////let bp = item.backpointers.into_inner();
            ////existent.backpointers.borrow_mut().extend(bp);
        //} else {
            //self.0.insert(Rc::new(item));
        //}
    //}

    //pub fn len(&self) -> usize {
        //self.0.len()
    //}

    //pub fn iter(&self) -> impl Iterator<Item=&Rc<Item>> {
        //self.0.iter()
    //}
//}

//impl Extend<Item> for StateSet {
    //fn extend<I: IntoIterator<Item=Item>>(&mut self, iterable: I) {
        //for item in iterable { self.insert(item); }
    //}
//}

//impl iter::FromIterator<Item> for StateSet {
    //fn from_iter<I: IntoIterator<Item=Item>>(iterable: I) -> Self {
        ////StateSet(HashSet::from_iter(iterable.into_iter()))
        //let mut ss = StateSet::default();
        //ss.extend(iterable.into_iter());
        //ss
    //}
//}

///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::collections::HashSet;
    use std::cell::RefCell;
    // TODO use super::{Rule, Item, Symbol, StateSet, Trigger};
    use super::{Rule, Item, Symbol, Trigger};

    fn gen_rule1() -> Rc<Rule> {
        fn testfn(o: &str) -> bool { o.len() == 1 && "+-".contains(o) }
        // S -> S +- d
        Rc::new(Rule::new("S", &[
            Symbol::new("S"),
            Symbol::new2("+-", testfn),
            Symbol::new2("d", |n| n.chars().all(|c| "123".contains(c))),
        ]))
    }

    fn gen_rule2() -> Rc<Rule> {
        fn testfn(o: &str) -> bool { o.len() == 1 && "*/".contains(o) }
        // S -> S */ d
        Rc::new(Rule::new("S", &[
            Symbol::new("S"),
            Symbol::new2("*/", testfn),
            Symbol::new2("d", |n| n.chars().all(|c| "123".contains(c))),
        ]))
    }

    fn item(rule: Rc<Rule>, dot: usize, start: usize, end: usize) -> Item {
        Item{rule, dot, start, end, backpointers: RefCell::new(HashSet::new())}
    }

    #[test]
    fn item_basics() {
        // Check item equality
        assert_eq!(item(gen_rule1(), 0, 0, 0), item(gen_rule1(), 0, 0, 0));
        assert_ne!(item(gen_rule2(), 0, 0, 0), item(gen_rule1(), 0, 0, 0));
        assert_ne!(item(gen_rule1(), 1, 0, 0), item(gen_rule1(), 0, 0, 0));
        // Check item complete
        assert!(!item(gen_rule2(), 2, 0, 5).complete());
        assert!(item(gen_rule2(), 3, 0, 4).complete());
        // Check next symbol
        assert!(item(gen_rule1(), 0, 0, 5).next_symbol().unwrap().nonterm().is_some());
        assert!(item(gen_rule1(), 2, 0, 5).next_symbol().unwrap().terminal().is_some());
    }

    #[test]
    fn item_predict() {
        let predict = Item::predict_new(&gen_rule1(), 23);
        assert_eq!(item(gen_rule1(), 0, 23, 23), predict);
        assert_eq!(predict.start, predict.end);
        assert_eq!(predict.sources().len(), 0);
    }

    #[test]
    fn item_scan() {
        // Source: S -> S . + d
        let source = Rc::new(item(gen_rule1(), 1, 0, 1));
        // Scan a '+' token
        let scan = Item::scan_new(&source, 2, "+");
        assert_eq!(item(gen_rule1(), 2, 0, 2), scan);
        // Check scan item backpointers
        let scan_src = scan.sources();
        assert!(scan_src.contains(&(source, Trigger::Scan("+".to_string()))));
        assert_eq!(scan_src.len(), 1);
    }

    #[test]
    fn item_complete() {
        // Input could be: 2 * 3 + 1
        // Source: S -> . S + d
        let source = Rc::new(item(gen_rule1(), 0, 0, 0));
        // A trigger reaches completion (2 * 3) - S -> S * d .
        let trigger = Rc::new(item(gen_rule2(), 0, 0, 3));
        // generate completion
        let complete_based = Item::complete_new(&source, &trigger, 3);
        assert_eq!(item(gen_rule1(), 1, 0, 3), complete_based);
        // Check completion item backpointers
        let src = complete_based.sources();
        assert!(src.contains(&(source, Trigger::Complete(trigger))));
        assert_eq!(src.len(), 1);
    }

    //#[test]
    //fn stateset_dedups() {
        ////check that items are deduped in statesets
        //let mut ss = StateSet::default();
        //// Insert equivalent item twice
        //ss.insert(item(gen_rule2(), 0, 0, 0));
        //ss.insert(item(gen_rule2(), 0, 0, 0));
        //assert_eq!(ss.len(), 1);
        //// Insert new item, and again
        //ss.insert(item(gen_rule2(), 1, 0, 1));
        //assert_eq!(ss.len(), 2);
        //ss.insert(item(gen_rule2(), 1, 0, 1));
        //assert_eq!(ss.len(), 2);
        //// Yet another different item
        //ss.insert(item(gen_rule2(), 2, 0, 1));
        //assert_eq!(ss.len(), 3);
    //}

    #[test]
    fn stateset_bp_merge() {
        // Source: S -> . S + d
        let source = Rc::new(item(gen_rule1(), 0, 0, 0));

        // rule3: S -> d
        let rule3 = Rc::new(Rule::new("S", &[
            Symbol::new2("d", |n| n.chars().all(|c| "123".contains(c))),
        ]));
        // S -> d .
        let trigger1 = Rc::new(item(rule3, 1, 0, 1));
        // S -> S . + d
        let complete1 = Item::complete_new(&source, &trigger1, 1);
        assert_eq!(complete1, item(gen_rule1(), 1, 0, 1));

        // A trigger2 reaches completion (2 * 3) - S -> S * d .
        let trigger2 = Rc::new(item(gen_rule2(), 0, 0, 3));
        // S -> S . * d
        let complete2 = Item::complete_new(&source, &trigger2, 3);
        assert_eq!(complete2, item(gen_rule1(), 1, 0, 3));

        // rule4: S -> hex
        let rule4 = Rc::new(Rule::new("S", &[Symbol::new2("hex", |n| n == "0x3")]));
        // S -> hex .
        let trigger3 = Rc::new(item(rule4, 1, 0, 1));
        // S -> S . + d
        let complete3 = Item::complete_new(&source, &trigger3, 1);
        assert_eq!(complete3, item(gen_rule1(), 1, 0, 1));


        // Check that complete1 and complete2 have been merged
        //let mut ss = StateSet::default();
        //ss.insert(complete1);
        //ss.insert(complete2);
        //ss.insert(complete3);
        //assert_eq!(ss.len(), 2);

        // Check complete1 / complete3 merged item
        //let complete13_sources = ss.0.get(&item(gen_rule1(), 1, 0, 1))
            //.unwrap()
            //.sources.borrow();
        //assert!(complete13_sources.contains(
                //&(source.clone(), Trigger::Complete(trigger1))));
        //assert!(complete13_sources.contains(
                //&(source, Trigger::Complete(trigger3))));
    }
}
