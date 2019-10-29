use itertools::Itertools;
use std::fmt::{self, Display};

/// SQL statment with boolean logic.
pub struct PredicateStatement<'s> {
    statement: &'static str,
    predicates: &'s [Box<dyn Display>],
}

impl fmt::Display for PredicateStatement<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.statement,
            self.predicates.iter().join("\nAND ")
        )
    }
}

/// Collection of boolean predicates.
pub struct Predicates(Vec<Box<dyn Display>>);

impl IntoIterator for Predicates {
    type Item = Box<dyn Display>;
    type IntoIter = std::vec::IntoIter<Box<dyn Display>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Predicates {
    /// Create empty collection.
    pub fn new() -> Predicates {
        Predicates(Vec::new())
    }

    /// Creates collection containing given predicate.
    pub fn from<S: Display + 'static>(predicate: S) -> Predicates {
        Predicates::new().and(predicate)
    }

    /// Crates collection containing given predicates.
    pub fn from_all<S, I, IT>(predicates: I) -> Self
    where
        S: Display + 'static,
        I: IntoIterator<Item = S, IntoIter = IT>,
        IT: Iterator<Item = S>,
    {
        Predicates::new().and_all(predicates)
    }

    /// Gets WHERE statement with predicates.
    pub fn as_where<'s>(&'s self) -> PredicateStatement<'s> {
        PredicateStatement {
            statement: "WHERE",
            predicates: &self.0,
        }
    }

    /// Appends predicate.
    pub fn and_push<S: Display + 'static>(&mut self, predicate: S) {
        self.and_extend(Some(predicate))
    }

    /// Appends all predicates.
    pub fn and_extend<S, I, IT>(&mut self, predicates: I) -> ()
    where
        S: Display + 'static,
        I: IntoIterator<Item = S, IntoIter = IT>,
        IT: Iterator<Item = S>,
    {
        self.0.extend(
            predicates
                .into_iter()
                .map(|c| Box::new(c) as Box<dyn Display>),
        );
    }

    /// Appends predicate with fluid API.
    pub fn and<S: Display + 'static>(mut self, predicate: S) -> Self {
        self.and_push(predicate);
        self
    }

    /// Appends all predicates with fluid API.
    pub fn and_all<S, I, IT>(mut self, predicates: I) -> Self
    where
        S: Display + 'static,
        I: IntoIterator<Item = S, IntoIter = IT>,
        IT: Iterator<Item = S>,
    {
        self.and_extend(predicates);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(
            Predicates::from("foo = 'bar'")
                .and("baz")
                .and_all(["hello", "world"].iter())
                .and_all(Predicates::from_all(["abc", "123"].iter()))
                .as_where()
                .to_string(),
            "WHERE foo = \'bar\'\nAND baz\nAND hello\nAND world\nAND abc\nAND 123"
        );
    }
}
