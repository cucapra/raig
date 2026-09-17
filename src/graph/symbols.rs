// Copyright 2026 Cornell University
// released under MIT License
// author: Kevin Laeufer <laeufer@cornell.edu>

use std::num::NonZeroU32;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SymbolKind {
    Output,
    Input,
    Latch,
    BadState,
    Constraint,
    Justice,
    Fairness,
}

impl SymbolKind {
    #[inline]
    pub const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            b'o' => Some(Self::Output),
            b'i' => Some(Self::Input),
            b'l' => Some(Self::Latch),
            b'b' => Some(Self::BadState),
            b'c' => Some(Self::Constraint),
            b'j' => Some(Self::Justice),
            b'f' => Some(Self::Fairness),
            _ => None,
        }
    }

    #[inline]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::Output => b'o',
            Self::Input => b'i',
            Self::Latch => b'l',
            Self::BadState => b'b',
            Self::Constraint => b'c',
            Self::Justice => b'j',
            Self::Fairness => b'f',
        }
    }

    #[inline]
    pub const fn is_tag(tag: u8) -> bool {
        matches!(tag, b'o' | b'i' | b'l' | b'b' | b'c' | b'j' | b'f')
    }
}

/// Symbol table for an AIG graph.
#[derive(Debug, Default)]
pub struct SymbolTable {
    strings: StringStorage,
    outputs: Vec<Option<SymbolInt>>,
    inputs: Vec<Option<SymbolInt>>,
    latches: Vec<Option<SymbolInt>>,
    bad_states: Vec<Option<SymbolInt>>,
    constraints: Vec<Option<SymbolInt>>,
    justice: Vec<Option<SymbolInt>>,
    fairness: Vec<Option<SymbolInt>>,
}

macro_rules! declare_access_fns {
    ($access_name:ident, $add_name:ident, $iter_name:ident, $vec_name:ident, $kind:path) => {
        pub fn $access_name(&self, index: usize) -> Option<&str> {
            if let Some(Some(symbol)) = self.$vec_name.get(index) {
                Some(self.strings.get_string(*symbol))
            } else {
                None
            }
        }

        pub fn $add_name(&mut self, index: usize, name: &str) {
            if self.$vec_name.len() <= index {
                self.$vec_name.resize(index + 1, None);
            }
            self.$vec_name[index] = Some(self.strings.add_string(name));
        }

        fn $iter_name(&self) -> impl Iterator<Item = (SymbolKind, usize, &str)> {
            self.$vec_name
                .iter()
                .enumerate()
                .flat_map(|(idx, maybe_sym)| {
                    maybe_sym.map(|sym| ($kind, idx, self.strings.get_string(sym)))
                })
        }
    };
}
impl SymbolTable {
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty() && self.inputs.is_empty() && self.latches.is_empty()
    }

    declare_access_fns!(input, name_input, iter_inputs, inputs, SymbolKind::Input);
    declare_access_fns!(latch, name_latch, iter_latches, latches, SymbolKind::Latch);
    declare_access_fns!(
        output,
        name_output,
        iter_outputs,
        outputs,
        SymbolKind::Output
    );
    declare_access_fns!(
        bad_state,
        name_bad_state,
        iter_bad_states,
        bad_states,
        SymbolKind::BadState
    );
    declare_access_fns!(
        constraint,
        name_constraint,
        iter_constraints,
        constraints,
        SymbolKind::Constraint
    );
    declare_access_fns!(
        justice,
        name_justice,
        iter_justice,
        justice,
        SymbolKind::Justice
    );
    declare_access_fns!(
        fairness,
        name_fairness,
        iter_fairness,
        fairness,
        SymbolKind::Fairness
    );

    /// Returns an iterator over all symbols in the following order:
    /// input -> latch -> output -> bad_states -> constraints -> justice -> fairness
    pub fn symbols(&self) -> impl Iterator<Item = (SymbolKind, usize, &str)> {
        self.iter_inputs()
            .chain(self.iter_latches())
            .chain(self.iter_outputs())
            .chain(self.iter_bad_states())
            .chain(self.iter_constraints())
            .chain(self.iter_justice())
            .chain(self.iter_fairness())
    }

    pub fn name_symbol(&mut self, kind: SymbolKind, index: usize, name: &str) {
        match kind {
            SymbolKind::Output => self.name_output(index, name),
            SymbolKind::Input => self.name_input(index, name),
            SymbolKind::Latch => self.name_latch(index, name),
            SymbolKind::BadState => self.name_bad_state(index, name),
            SymbolKind::Constraint => self.name_constraint(index, name),
            SymbolKind::Justice => self.name_constraint(index, name),
            SymbolKind::Fairness => self.name_fairness(index, name),
        }
    }
}

/// Efficient immutable string storage that avoid allocations.
#[derive(Debug, Default)]
struct StringStorage {
    ends: Vec<OffsetInt>,
    bytes: Vec<u8>,
}
type OffsetInt = u32;
type SymbolInt = NonZeroU32;

impl StringStorage {
    const EMPTY_STR: SymbolInt = SymbolInt::new(1).unwrap();

    fn add_string(&mut self, value: &str) -> SymbolInt {
        if value.is_empty() {
            Self::EMPTY_STR
        } else {
            self.bytes.extend_from_slice(value.as_bytes());
            self.ends.push(self.bytes.len() as OffsetInt);
            SymbolInt::new((self.ends.len() + 1) as u32).unwrap()
        }
    }

    fn get_string(&self, sym: SymbolInt) -> &str {
        if sym == Self::EMPTY_STR {
            ""
        } else {
            let index = (sym.get() - 2) as usize;
            let start = if index == 0 { 0 } else { self.ends[index - 1] } as usize;
            let end = self.ends[index] as usize;
            unsafe { core::str::from_utf8_unchecked(&self.bytes[start..end]) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_storage() {
        let strings = ["", "123", "abc", "😭", "🏊🏾‍♂️🏊🏾‍♂️"];
        let mut st = StringStorage::default();
        let symbols: Vec<_> = strings.iter().map(|s| st.add_string(s)).collect();
        for (&sym_id, &value) in symbols.iter().zip(strings.iter()).rev() {
            assert_eq!(st.get_string(sym_id), value);
        }
    }
}
