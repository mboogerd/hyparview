fn main() {
    println!("Hello, world!");
}

pub mod bounded_set;

#[cfg(test)]
mod passive_view_spec {}

#[cfg(test)]
mod active_view_spec {

    #[test]
    fn hello() {}
}

#[cfg(test)]
mod join_spec {}

#[cfg(test)]
mod loquat_spec {}
