use crate::{Server, ServerBalancer};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

struct Node<S: Server> {
    server: Arc<S>,
    prev: Option<usize>,
    next: Option<usize>,
}

struct LinkedList<S: Server> {
    indexes: HashMap<S::Id, usize>,
    nodes: Vec<Option<Node<S>>>,

    head: Option<usize>,
    tail: Option<usize>,

    cursor: Option<usize>,

    free: Vec<usize>,
}

impl<S: Server> Default for LinkedList<S> {
    fn default() -> Self {
        Self {
            indexes: HashMap::new(),
            nodes: Vec::new(),
            head: None,
            tail: None,
            cursor: None,
            free: Vec::new(),
        }
    }
}

impl<S: Server> LinkedList<S> {
    // push new server to the end of the linked list
    fn push_back(&mut self, server: S) -> bool {
        // if server already present then exit
        if self.indexes.contains_key(server.id()) {
            return false;
        }

        let id = server.id().clone();

        // allocate node and calc index
        let index = self.alloc(Node {
            server: Arc::new(server),
            prev: self.tail,
            next: None,
        });

        // move tail if list not empty,
        // or set head if list empty
        match self.tail {
            Some(tail) => self.nodes[tail].as_mut().expect("missing tail").next = Some(index),
            None => self.head = Some(index),
        }
        self.tail = Some(index);

        self.cursor.get_or_insert(index);
        self.indexes.insert(id, index);
        true
    }

    fn remove_by_id(&mut self, id: &S::Id) -> Option<Arc<S>> {
        let index = self.indexes.remove(id)?;

        let node = self.nodes[index].take().expect("missing node");

        self.free.push(index);

        match node.prev {
            Some(prev) => self.nodes[prev].as_mut().expect("missing prev").next = node.next,
            None => self.head = node.next,
        }
        match node.next {
            Some(next) => self.nodes[next].as_mut().expect("missing next").prev = node.prev,
            None => self.tail = node.prev,
        }

        if self.cursor == Some(index) {
            self.cursor = node.next.or(self.head);
        }

        Some(node.server)
    }

    fn next_server(&mut self) -> Option<Arc<S>> {
        let index = self.cursor?;
        let node = self.nodes[index].as_ref().expect("missing cursor node");
        let server = Arc::clone(&node.server);
        self.cursor = node.next.or(self.head);
        Some(server)
    }

    fn alloc(&mut self, node: Node<S>) -> usize {
        if let Some(index) = self.free.pop() {
            // use vacant position at the vec
            self.nodes[index] = Some(node);
            index
        } else {
            // use new position
            self.nodes.push(Some(node));
            self.nodes.len() - 1
        }
    }
}

pub struct StableRoundRobinBalancer<S: Server> {
    list: Mutex<LinkedList<S>>,
}

impl<S: Server> Default for StableRoundRobinBalancer<S> {
    fn default() -> Self {
        Self {
            list: Mutex::new(LinkedList::default()),
        }
    }
}

impl<S: Server> StableRoundRobinBalancer<S> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S: Server> ServerBalancer<S> for StableRoundRobinBalancer<S> {
    async fn add_server(&self, server: S) -> bool {
        self.list.lock().push_back(server)
    }

    async fn remove_server(&self, id: &S::Id) -> Option<Arc<S>> {
        self.list.lock().remove_by_id(id)
    }

    async fn get_next_server(&self, _context: &()) -> Option<Arc<S>> {
        self.list.lock().next_server()
    }
}
