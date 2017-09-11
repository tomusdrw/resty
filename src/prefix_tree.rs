use std::mem;
use arrayvec::ArrayVec;

enum Node<T> {
    Empty,
    Data(T),
    Tree(Option<T>, Box<Tree<T>>),
}

const SIZE: usize = 256;

// TODO [ToDr] Debug
pub struct Tree<T> {
    routes: [Node<T>; SIZE],
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        let mut routes = ArrayVec::new();
        for i in 0..SIZE {
            routes.insert(i, Node::Empty);
        }
        Tree {
            routes: match routes.into_inner() {
                Ok(ok) => ok,
                Err(_) => unreachable!(),
            },
        }
    }
}

fn merge_nodes<T>(me: Node<T>, other: Node<T>) -> Node<T> {
    match (me, other) {
        (Node::Empty, any) => any,
        (any, Node::Empty) => any,
        (Node::Data(_), Node::Data(d)) => Node::Data(d),
        (Node::Data(a), Node::Tree(b, next)) => Node::Tree(b.or(Some(a)), next),
        (Node::Tree(_, next), Node::Data(d)) => Node::Tree(Some(d), next),
        (Node::Tree(a, mut next), Node::Tree(b, mut next2)) => {
            merge_trees(&mut next, &mut next2);
            Node::Tree(b.or(a), next)
        },
    }
}

fn merge_trees<T>(me: &mut Tree<T>, other: &mut Tree<T>) {
    for i in 0..SIZE {
        let old_me = mem::replace(&mut me.routes[i], Node::Empty);
        let old_other = mem::replace(&mut other.routes[i], Node::Empty);
        me.routes[i] = merge_nodes(old_me, old_other);
    }
}

impl<T> Tree<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge<K: AsRef<[u8]>>(&mut self, prefix: K, mut other: Tree<T>) {
        let bytes = prefix.as_ref();
        if bytes.is_empty() {
            merge_trees(self, &mut other);
            return;
        }

        let b = bytes[0] as usize;
        let old = mem::replace(&mut self.routes[b], Node::Empty);
        self.routes[b] = match old {
            Node::Empty => Node::Tree(None, Box::new(Tree::new())),
            Node::Data(d) => Node::Tree(Some(d), Box::new(Tree::new())),
            Node::Tree(d, tree) => Node::Tree(d, tree),
        };

        if let Node::Tree(_, ref mut tree) = self.routes[b] {
            tree.merge(&bytes[1..], other);
        }
    }

    pub fn remove<K: AsRef<[u8]>>(&mut self, key: K) -> Option<T> {
        let bytes = key.as_ref();
        let len = bytes.len();
        assert!(len > 0, "Empty keys are not supported.");

        let is_last = len == 1;
        let b = bytes[0] as usize;
        let old = mem::replace(&mut self.routes[b], Node::Empty);
        if is_last {
            let (new, old) = match old {
                Node::Empty => (Node::Empty, None),
                Node::Data(d) => (Node::Empty, Some(d)),
                Node::Tree(d, tree) => (Node::Tree(None, tree), d),
            };
            self.routes[b] = new;
            return old;
        }

        self.routes[b] = match old {
            Node::Empty => return None,
            Node::Data(d) => return Some(d),
            Node::Tree(d, tree) => Node::Tree(d, tree),
        };

        if let Node::Tree(_, ref mut tree) = self.routes[b] {
            tree.remove(&bytes[1..])
        } else {
            None
        }
    }

    pub fn insert<K: AsRef<[u8]>>(&mut self, key: K, value: T) -> Option<T> {
        let bytes = key.as_ref();
        let len = bytes.len();
        assert!(len > 0, "Empty keys are not supported.");

        let is_last = len == 1;
        let b = bytes[0] as usize;
        let old = mem::replace(&mut self.routes[b], Node::Empty);
        if is_last {
            let (new, old) = match old {
                Node::Empty => (Node::Data(value), None),
                Node::Data(d) => (Node::Data(value), Some(d)),
                Node::Tree(d, tree) => (Node::Tree(Some(value), tree), d),
            };
            self.routes[b] = new;
            return old;
        }

        self.routes[b] = match old {
            Node::Empty => Node::Tree(None, Box::new(Tree::new())),
            Node::Data(d) => Node::Tree(Some(d), Box::new(Tree::new())),
            Node::Tree(d, tree) => Node::Tree(d, tree),
        };

        if let Node::Tree(_, ref mut tree) = self.routes[b] {
            tree.insert(&bytes[1..], value)
        } else {
            None
        }
    }

    /// Finds the first terminal element by looking at the prefix.
    /// Returns the length of the matched prefix and a reference to the element.
    pub fn find<K: AsRef<[u8]>>(&self, key: K) -> Option<(usize, &T)> {
        let bytes = key.as_ref();


        let mut best_result = None;
        let mut current = &self.routes;
        for (pos, byte) in bytes.iter().enumerate() {
            match current[*byte as usize] {
                // final node
                Node::Empty => return best_result,
                // Save it as best result, but look for longer pattern
                Node::Data(ref t) => best_result = Some((pos + 1, t)),
                // Descend in the tree
                Node::Tree(ref top_level, ref tree) => {
                    if let Some(ref top_level) = *top_level {
                        best_result = Some((pos + 1, top_level));
                    }
                    current = &tree.routes
                },
            }
        }

        return best_result;
    }
}

#[cfg(test)]
mod tests {
    use super::Tree;

    #[test]
    fn should_find_exact_match() {
        let mut tree = Tree::new();

        tree.insert("abc", 5);

        assert_eq!(tree.find("ab"), None);
        assert_eq!(tree.find("abc"), Some((3, &5)));
        assert_eq!(tree.find("abcd"), Some((3, &5)));
    }

    #[test]
    fn should_merge_two_trees() {
        let mut tree1 = Tree::new();
        tree1.insert("abc", 4);
        tree1.insert("axy", 9);
        tree1.insert("z", 6);
        let mut tree2 = Tree::new();
        tree2.insert("b", 5);
        tree2.insert("abc", 7);
        tree2.insert("xyz", 10);

        tree1.merge("a", tree2);

        assert_eq!(tree1.find("ab"), Some((2, &5)));
        assert_eq!(tree1.find("abc"), Some((3, &4)));
        assert_eq!(tree1.find("abcd"), Some((3, &4)));
        assert_eq!(tree1.find("aabcd"), Some((4, &7)));
        assert_eq!(tree1.find("axy"), Some((3, &9)));
        assert_eq!(tree1.find("axyz"), Some((4, &10)));
        assert_eq!(tree1.find("axyzx"), Some((4, &10)));
        assert_eq!(tree1.find("z"), Some((1, &6)));
    }
}
