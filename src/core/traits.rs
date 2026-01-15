use std::fmt;

pub trait AstNode: fmt::Debug + Clone {
    fn node_type(&self) -> &'static str;

    fn has_children(&self) -> bool {
        false
    }
}

pub trait Named {
    fn name(&self) -> Option<&str>;
}

pub trait ToSource {
    fn to_source(&self) -> String;
}
