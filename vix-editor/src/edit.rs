/// A single reversible edit. Both `Insert` and `Delete` carry enough
/// information to undo themselves without re-computing anything.
#[derive(Debug, Clone)]
pub enum Op {
    Insert {
        at: usize,   // byte offset in the rope
        text: String,
    },
    Delete {
        at: usize,
        text: String, // the text that was removed (needed for undo)
    },
}

impl Op {
    /// Return the inverse of this operation.
    pub fn inverse(&self) -> Op {
        match self {
            Op::Insert { at, text } => Op::Delete { at: *at, text: text.clone() },
            Op::Delete { at, text } => Op::Insert { at: *at, text: text.clone() },
        }
    }
}

/// A linear undo/redo stack.
/// Every committed edit pushes onto `past`; undo pops from `past` onto `future`.
#[derive(Default)]
pub struct EditLog {
    past:   Vec<Op>,
    future: Vec<Op>,
}

impl EditLog {
    pub fn push(&mut self, op: Op) {
        self.past.push(op);
        self.future.clear(); // new edit invalidates redo history
    }

    pub fn undo(&mut self) -> Option<Op> {
        let op = self.past.pop()?;
        let inv = op.inverse();
        self.future.push(op);
        Some(inv)
    }

    pub fn redo(&mut self) -> Option<Op> {
        let op = self.future.pop()?;
        let fwd = op.inverse();
        self.past.push(op);
        Some(fwd)
    }
}
