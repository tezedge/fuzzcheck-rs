use fuzzcheck::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub enum X {
    A(u8),
}

#[cfg(test)]
mod test {
    use super::*;
    use fuzzcheck::Mutator;
    #[test]
    #[no_coverage]
    fn test_compile() {
        let m = X::default_mutator();
        let (_value, _): (X, _) = m.random_arbitrary(10.0);
    }
}
